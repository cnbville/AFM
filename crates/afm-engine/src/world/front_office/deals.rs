//! Trades at your club (lane 10): offers you make to another club's GM,
//! calls that come in from the AI's own trade windows, shopping a player
//! around the league, your trading block, and conditional picks. Every
//! answer comes from the other GM's own valuation (`trades::player_points`
//! and `pick_value`: his read of the players, his chart, his mode and his
//! urgency), the same numbers his AI trades use, and an executed trade
//! keeps the cap rules the AI's do (the seller keeps the bonus).

use crate::calendar::{Date, Kind, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::pro::{self, DeadWhy, Status, money, trades};

use super::{Duty, Owner};

/// A draft pick as a trade names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub year: i32,
    pub round: u32,
    /// The club whose pick it was originally (the earner of a comp pick).
    pub orig: usize,
    /// A compensatory pick (tradable from 2017, real).
    #[serde(default)]
    pub comp: bool,
}

/// A condition on a pick (real deals: "a 2027 third that becomes a second
/// if he plays 60% of the snaps"): met, the pick moves up a round.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Condition {
    /// He dresses for at least this many games.
    Games(u32),
    /// He starts at least this many games.
    Starts(u32),
    /// He makes the Pro Bowl.
    ProBowl,
}

impl Condition {
    pub fn words(self) -> String {
        match self {
            Condition::Games(n) => format!("if he plays in {n} games"),
            Condition::Starts(n) => format!("if he starts {n} games"),
            Condition::ProBowl => "if he makes the Pro Bowl".into(),
        }
    }
}

/// One side of a trade: players by id and picks, with an optional
/// condition on one pick tied to a player the other side receives.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Side {
    pub players: Vec<u32>,
    pub picks: Vec<Pick>,
}

/// A trade offer between your club and another.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    pub other: usize,
    /// What you send.
    pub give: Side,
    /// What you get.
    pub get: Side,
    /// A condition on one of your picks: (the pick, the condition, the
    /// player it reads, a player you receive).
    #[serde(default)]
    pub conditional: Option<(Pick, Condition, u32)>,
}

/// A conditional pick on the books, settled after the season.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct Conditional {
    pub pick: Pick,
    pub from: usize,
    pub to: usize,
    pub player: u32,
    pub cond: Condition,
    /// The season the condition reads.
    pub season: i32,
}

/// A call from another club about one of your players.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Call {
    pub date: Date,
    pub until: Date,
    pub buyer: usize,
    pub player: u32,
    pub picks: Vec<Pick>,
    pub answered: bool,
}

/// Your trade desk.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct TradeDesk {
    pub calls: Vec<Call>,
    pub conditionals: Vec<Conditional>,
    /// Your trading block: players you let the league know are available.
    pub block: Vec<u32>,
    /// Trades you made (their text, newest last).
    pub made: Vec<(Date, String)>,
}

/// The other GM's answer.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Answer {
    pub accept: bool,
    pub text: String,
    /// What he'd want added from your side to say yes.
    pub counter: Option<Vec<Pick>>,
    /// Your chart's points for each side (what you give, what you get).
    pub points: (f64, f64),
}

/// Whether trades can be made today: from the new league year to the
/// trade deadline (after it, deals wait for the next league year).
pub fn window_open(w: &World) -> bool {
    w.today >= w.dates.league_year.date && w.today <= w.dates.trade_deadline.date
}

/// The first draft whose picks can still be traded.
fn first_draft(w: &World) -> i32 {
    if w.today < w.dates.draft[0].date { w.year } else { w.year + 1 }
}

/// Every pick a club owns over the next three drafts (comp picks of the
/// coming draft from 2017).
pub fn picks_of(w: &World, t: usize) -> Vec<Pick> {
    let first = first_draft(w);
    let teams = w.nfl.len();
    let mut v = Vec::new();
    for y in first..first + 3 {
        let rounds = crate::world::franchise::draft_rounds(y);
        for round in 1..=rounds {
            for orig in 0..teams {
                if w.market.picks.owner_of(y, round, orig) == t
                    && crate::world::franchise::active(orig, y.min(w.year + 1))
                {
                    v.push(Pick { year: y, round, orig, comp: false });
                }
            }
        }
    }
    if first >= 2017 {
        for c in w.market.picks.comp.iter().filter(|c| c.owner == t && c.year == first) {
            v.push(Pick { year: c.year, round: c.round, orig: c.team, comp: true });
        }
    }
    v
}

fn owns(w: &World, t: usize, p: &Pick) -> bool {
    if p.comp {
        w.market.picks.comp.iter().any(|c| c.year == p.year && c.round == p.round && c.team == p.orig && c.owner == t)
    } else {
        w.market.picks.owner_of(p.year, p.round, p.orig) == t
    }
}

/// A pick's value on club `t`'s chart (a comp pick at the end of its round).
pub fn pick_points(w: &World, t: usize, p: &Pick) -> f64 {
    if p.comp {
        trades::pick_value(w, t, p.year, p.round + 1, p.orig) * 1.15
    } else {
        trades::pick_value(w, t, p.year, p.round, p.orig)
    }
}

/// A player's trade value to club `t` (his surplus over his deal).
pub fn player_points(w: &World, t: usize, i: usize) -> f64 {
    trades::player_points(w, t, i)
}

/// How likely a condition is met, as the GM sees it: a starter plays and
/// starts, a backup may not.
fn met_chance(w: &World, t: usize, i: usize, c: Condition) -> f64 {
    let pos = w.people[i].player.position;
    let slots = crate::league::depth_slots(pos);
    let better = w.clubs[t]
        .depth
        .get(pos.index())
        .map_or(0, |d| d.iter().filter(|&&j| j != i && pro::grade(w, t, j) > pro::grade(w, t, i)).count());
    let starter = better < slots.starters.max(1);
    let healthy = 0.85;
    match c {
        Condition::Games(n) => {
            if starter {
                healthy
            } else {
                clamp(0.6 - 0.02 * n as f64, 0.15, 0.6)
            }
        }
        Condition::Starts(n) => {
            if starter {
                healthy * clamp(1.2 - 0.04 * n as f64, 0.3, 1.0)
            } else {
                clamp(0.3 - 0.015 * n as f64, 0.05, 0.3)
            }
        }
        Condition::ProBowl => clamp((crate::world::pro::market::consensus(w, i) - 78.0) / 20.0, 0.02, 0.6),
    }
}

/// The value of a side to club `t`.
fn side_points(w: &World, t: usize, s: &Side, at: &std::collections::HashMap<u32, usize>) -> f64 {
    let players: f64 = s.players.iter().filter_map(|id| at.get(id)).map(|&i| player_points(w, t, i)).sum();
    let picks: f64 = s.picks.iter().map(|p| pick_points(w, t, p)).sum();
    players + picks
}

/// Checks a proposal against the rules: the window, who owns what, the
/// no-trade clauses, the injured, both caps after the deal.
fn check(w: &World, me: usize, p: &Proposal, at: &std::collections::HashMap<u32, usize>) -> Result<(), String> {
    if !window_open(w) {
        return Err(if w.today > w.dates.trade_deadline.date {
            "The trade deadline has passed. Trades open again with the new league year.".into()
        } else {
            "Trades open with the new league year.".into()
        });
    }
    let o = p.other;
    if o == me || !crate::world::franchise::active(o, w.year) {
        return Err("Pick another club.".into());
    }
    if p.give.players.is_empty() && p.give.picks.is_empty() || p.get.players.is_empty() && p.get.picks.is_empty() {
        return Err("A trade needs something going each way.".into());
    }
    for (side, t) in [(&p.give, me), (&p.get, o)] {
        for id in &side.players {
            let &i = at.get(id).ok_or("A player in the deal is gone.")?;
            let c = w.people[i].pro.as_ref().ok_or("Not a pro.")?;
            if c.team != Some(t) {
                return Err(format!("{} isn't on the {}.", w.people[i].name, w.nfl[t].name));
            }
            if c.status == Status::Reserve || c.out_weeks > 0 {
                return Err(format!("{} is hurt: no club takes on an injured player.", w.people[i].name));
            }
            if c.contract.as_ref().is_none_or(|k| k.team != t || k.last_year() < w.year) {
                return Err(format!("{} has no contract to trade.", w.people[i].name));
            }
        }
        for pk in &side.picks {
            if !owns(w, t, pk) {
                return Err(format!("The {} don't own that pick.", w.nfl[t].name));
            }
            if pk.comp && pk.year < 2017 {
                return Err("Compensatory picks can't be traded before 2017.".into());
            }
        }
    }
    if let Some((pk, _, pid)) = &p.conditional {
        if !p.give.picks.contains(pk) || !p.get.players.contains(pid) {
            return Err("A condition sits on one of your picks and reads a player you receive.".into());
        }
        if pk.round <= 1 {
            return Err("A first-round pick can't move up a round.".into());
        }
        if pk.year <= w.year {
            return Err("A conditional pick must be in a later draft than this season.".into());
        }
    }
    // Both caps after the deal (the seller keeps the bonus).
    let year = w.year;
    if money::capped(year) {
        let june1 = pro::after_june1(w, w.today);
        for (t, out, inn) in [(me, &p.give, &p.get), (o, &p.get, &p.give)] {
            let mut delta = 0.0;
            for id in &out.players {
                let k = w.people[at[id]].pro.as_ref().unwrap().contract.as_ref().unwrap();
                delta -= k.cap_hit(year);
                delta += trades::kept_bonus(k, year, june1).0;
            }
            for id in &inn.players {
                let k = w.people[at[id]].pro.as_ref().unwrap().contract.as_ref().unwrap();
                delta += k.cap_hit(year) - k.year(year).map_or(0.0, |x| x.proration);
            }
            // You answer to the cap; the other GM keeps his season's reserve.
            let room = if t == me { pro::cap_room(w, t, year) } else { pro::space(w, t, year) };
            if delta > room + 1.0 {
                return Err(format!(
                    "The {} would be {} over the cap after the deal.",
                    w.nfl[t].name,
                    super::dollars(delta - room)
                ));
            }
        }
    }
    Ok(())
}

fn ids(p: &Proposal) -> Vec<u32> {
    p.give.players.iter().chain(p.get.players.iter()).copied().collect()
}

/// What the other GM needs: his gain on his own chart must clear a margin
/// (a GM fleeced by you before asks more, a GM who fleeced you less).
fn margin(w: &World, o: usize, me: usize) -> f64 {
    let fleece = w.market.fleece.get(o).and_then(|v| v.get(me)).copied().unwrap_or(0.0);
    40.0 + (fleece / 5000.0).clamp(0.0, 0.3) * 400.0
}

/// The other GM's answer to your offer, without making it.
pub fn evaluate(w: &World, p: &Proposal) -> Result<Answer, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let at = super::indices(w, &ids(p));
    check(w, me, p, &at)?;
    let o = p.other;
    let mut gets = side_points(w, o, &p.give, &at);
    // A conditional pick is worth what it's likely to become.
    if let Some((pk, cond, pid)) = p.conditional
        && let Some(&i) = at.get(&pid)
    {
        let up = Pick { round: pk.round - 1, ..pk };
        let chance = met_chance(w, me, i, cond);
        gets += chance * (pick_points(w, o, &up) - pick_points(w, o, &pk));
    }
    let gives = side_points(w, o, &p.get, &at);
    let need = margin(w, o, me);
    let yours = (side_points(w, me, &p.give, &at), side_points(w, me, &p.get, &at));
    // No-trade clauses: a player can waive his for a better club.
    for id in &p.get.players {
        let i = at[id];
        if w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| k.no_trade)
            && w.nfl[me].strength() < w.nfl[o].strength() + 2.0
        {
            return Ok(Answer {
                accept: false,
                text: format!("{} has a no-trade clause and won't waive it to come to you.", w.people[i].name),
                counter: None,
                points: yours,
            });
        }
    }
    let gm = &w.clubs[o].office.gm;
    let gm_name = gm.name.clone();
    let team = w.nfl[o].name;
    if gets - gives >= need {
        return Ok(Answer {
            accept: true,
            text: format!("{gm_name}, the {team} GM: \"We can do that.\""),
            counter: None,
            points: yours,
        });
    }
    // Short: the cheapest of your picks (on your own chart) that closes the gap.
    let short = need - (gets - gives);
    let mine: Vec<Pick> = picks_of(w, me).into_iter().filter(|x| !p.give.picks.contains(x)).collect();
    let mut best: Option<(Vec<Pick>, f64)> = None;
    for a in 0..mine.len() {
        let va = pick_points(w, o, &mine[a]);
        let ca = pick_points(w, me, &mine[a]);
        if va >= short && best.as_ref().is_none_or(|b| ca < b.1) {
            best = Some((vec![mine[a]], ca));
        }
        for b in a + 1..mine.len() {
            let vb = va + pick_points(w, o, &mine[b]);
            let cb = ca + pick_points(w, me, &mine[b]);
            if vb >= short && best.as_ref().is_none_or(|x| cb < x.1) {
                best = Some((vec![mine[a], mine[b]], cb));
            }
        }
    }
    let far = gives > 0.0 && gets < gives * 0.5;
    Ok(match best {
        Some((extra, _)) if !far => {
            let words: Vec<String> = extra.iter().map(|x| pick_words(w, x)).collect();
            Answer {
                accept: false,
                text: format!("{gm_name}, the {team} GM: \"Add {} and we have a deal.\"", words.join(" and ")),
                counter: Some(extra),
                points: yours,
            }
        }
        _ => Answer {
            accept: false,
            text: format!(
                "{gm_name}, the {team} GM: \"{}\"",
                if far {
                    "That isn't close. We'd be giving you a lot more than we get."
                } else {
                    "We'd need more than you can put together in picks."
                }
            ),
            counter: None,
            points: yours,
        },
    })
}

/// "the Bears' 2027 round 2 pick", "your own 2027 round 4 pick".
pub fn pick_words(w: &World, p: &Pick) -> String {
    let whose = format!("{} ", pro::possessive(w.nfl[p.orig].name));
    format!("the {whose}{} {}-round {}pick", p.year, round_word(p.round), if p.comp { "compensatory " } else { "" })
}

/// A draft round as the wire writes it ("fourth").
pub fn round_word(r: u32) -> &'static str {
    ["first", "second", "third", "fourth", "fifth", "sixth", "seventh", "eighth", "ninth", "tenth", "11th", "12th"]
        .get(r.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("late")
}

/// Makes the trade if the other GM says yes.
pub fn propose(w: &mut World, p: &Proposal) -> Result<Answer, String> {
    let a = evaluate(w, p)?;
    if !a.accept {
        return Ok(a);
    }
    let me = super::club(w).unwrap();
    w.front_office.set_owner(Duty::Trades, Owner::You);
    execute(w, me, p);
    Ok(a)
}

/// Executes a trade between your club and another: players move with
/// their contracts, the seller keeps each player's bonus on its cap, picks
/// change hands, a conditional pick goes on the books.
fn execute(w: &mut World, me: usize, p: &Proposal) {
    let d = w.today;
    let year = w.year;
    let june1 = pro::after_june1(w, d);
    let o = p.other;
    let at = super::indices(w, &ids(p));
    // The text, as the league wire runs it, written before anyone moves
    // (each player with the club he leaves).
    let side_words = |w: &World, s: &Side, giver: usize| -> String {
        let mut v: Vec<String> = s.players.iter().filter_map(|id| at.get(id)).map(|&i| pro::describe(w, i)).collect();
        v.extend(s.picks.iter().map(|x| {
            if x.orig == giver {
                format!("a {} {}-round {}pick", x.year, round_word(x.round), if x.comp { "compensatory " } else { "" })
            } else {
                pick_words(w, x)
            }
        }));
        match v.len() {
            0 => String::new(),
            1 => v[0].clone(),
            _ => {
                let last = v.pop().unwrap();
                format!("{} and {last}", v.join(", "))
            }
        }
    };
    let cond = p.conditional.map_or(String::new(), |(pk, c, _)| {
        format!(
            " (the {} {}-round pick becomes a {}-round pick {})",
            pk.year,
            round_word(pk.round),
            round_word(pk.round - 1),
            c.words()
        )
    });
    let text = format!(
        "The {} trade {} to the {} for {}{cond}.",
        w.nfl[me].name,
        side_words(w, &p.give, me),
        w.nfl[o].name,
        side_words(w, &p.get, o)
    );
    let mut moved: Vec<(usize, usize, usize)> = Vec::new();
    for (side, from, to) in [(&p.give, me, o), (&p.get, o, me)] {
        for id in &side.players {
            moved.push((at[id], from, to));
        }
    }
    for &(i, from, to) in &moved {
        let (now, later) = w.people[i]
            .pro
            .as_ref()
            .and_then(|c| c.contract.as_ref())
            .map_or((0.0, 0.0), |k| trades::kept_bonus(k, year, june1));
        let id = w.people[i].player.id;
        w.clubs[from].add_dead(year, now, id, DeadWhy::Traded);
        w.clubs[from].add_dead(year + 1, later, id, DeadWhy::Traded);
        if let Some(k) = w.people[i].pro.as_mut().unwrap().contract.as_mut() {
            for y in k.years.iter_mut().filter(|y| y.year >= year) {
                y.proration = 0.0;
            }
        }
        pro::transfer(w, i, to, "traded");
        let c = w.people[i].pro.as_mut().unwrap();
        c.wants_out = false;
        c.hold = pro::market::Hold::None;
        c.status = Status::Roster;
    }
    for (side, to) in [(&p.give, o), (&p.get, me)] {
        for pk in &side.picks {
            move_pick(w, pk, to);
        }
    }
    if let Some((pk, cond, pid)) = p.conditional {
        w.front_office.trades.conditionals.push(Conditional {
            pick: pk,
            from: me,
            to: o,
            player: pid,
            cond,
            season: year,
        });
    }
    let big = moved.iter().any(|&(i, _, _)| pro::market::consensus(w, i) >= 82.0);
    let loud = if big { Loudness::Big } else { Loudness::Medium };
    let mut e =
        crate::calendar::Event::new(d, crate::calendar::Level::Nfl, Kind::Transfer, loud, text.clone()).team(me);
    for &(i, _, _) in &moved {
        let p = &w.people[i];
        e = e.person(p.player.id, &p.name);
    }
    for (t, club) in w.nfl.iter().enumerate() {
        e = e.club_link(t, club.name);
    }
    w.calendar.post(e);
    let my = w.market.year_mut(year);
    my.trades += 1;
    my.picks_traded += (p.give.picks.len() + p.get.picks.len()) as u32;
    if d > w.dates.nfl_kickoff {
        w.market.year_mut(year).season_trades += 1;
    }
    let player = p.get.players.first().or(p.give.players.first()).copied().unwrap_or(0);
    let picks: Vec<(i32, u32, usize)> = p.give.picks.iter().map(|x| (x.year, x.round, x.orig)).collect();
    w.market.picks.trades.push(trades::Trade { date: d, buyer: me, seller: o, player, picks, text: text.clone() });
    w.front_office.trades.made.push((d, text.clone()));
    if w.front_office.trades.made.len() > 60 {
        w.front_office.trades.made.remove(0);
    }
    w.front_office.trades.block.retain(|id| !p.give.players.contains(id));
    super::note(w, text, Some(player));
}

fn move_pick(w: &mut World, pk: &Pick, to: usize) {
    if pk.comp {
        if let Some(c) =
            w.market.picks.comp.iter_mut().find(|c| c.year == pk.year && c.round == pk.round && c.team == pk.orig)
        {
            c.owner = to;
        }
    } else {
        w.market.picks.owner.insert((pk.year, pk.round, pk.orig), to);
    }
}

/// What each club would give for one of your players, in picks: the best
/// package on your chart that costs the other club no more than he is
/// worth to it. Shopping a player leaks now and then (he hears it).
pub fn shop(w: &mut World, id: u32) -> Result<Vec<(usize, Vec<Pick>, f64)>, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if !window_open(w) {
        return Err("Trades are closed until the new league year.".into());
    }
    let year = w.year;
    let mut out = Vec::new();
    for o in crate::world::franchise::active_teams(year) {
        if o == me {
            continue;
        }
        let worth = player_points(w, o, i) - margin(w, o, me);
        if worth <= 30.0 {
            continue;
        }
        let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
        if money::capped(year) && k.is_some_and(|k| k.cap_hit(year) > pro::space(w, o, year)) {
            continue;
        }
        let theirs = picks_of(w, o);
        let mut best: Option<(Vec<Pick>, f64)> = None;
        for a in 0..theirs.len() {
            let ca = pick_points(w, o, &theirs[a]);
            let va = pick_points(w, me, &theirs[a]);
            if ca <= worth && best.as_ref().is_none_or(|b| va > b.1) {
                best = Some((vec![theirs[a]], va));
            }
            for b in a + 1..theirs.len() {
                let cb = ca + pick_points(w, o, &theirs[b]);
                let vb = va + pick_points(w, me, &theirs[b]);
                if cb <= worth && best.as_ref().is_none_or(|x| vb > x.1) {
                    best = Some((vec![theirs[a], theirs[b]], vb));
                }
            }
        }
        if let Some((picks, v)) = best {
            out.push((o, picks, v));
        }
    }
    out.sort_by(|a, b| b.2.total_cmp(&a.2));
    out.truncate(8);
    trades::shopping_rumor(w, i, me, w.today);
    Ok(out)
}

/// Puts a player on (or takes him off) your trading block: the AI's
/// buyers in each trade window treat him as available.
pub fn set_block(w: &mut World, id: u32, on: bool) {
    w.front_office.trades.block.retain(|&x| x != id);
    if on {
        w.front_office.trades.block.push(id);
    }
}

/// Whether a player of your club is on your block (read by the AI's trade window).
pub(crate) fn on_block(w: &World, t: usize, i: usize) -> bool {
    super::club(w) == Some(t) && w.front_office.trades.block.contains(&w.people[i].player.id)
}

/// An AI club's call about one of your players, from its own trade window:
/// it waits for your answer for two days (the deadline's the same day).
pub(crate) fn call_in(w: &mut World, d: Date, buyer: usize, _seller: usize, i: usize, package: Vec<(i32, u32, usize)>) {
    let id = w.people[i].player.id;
    if w.front_office.trades.calls.iter().any(|c| c.player == id && c.buyer == buyer && !c.answered) {
        return;
    }
    let until = if d >= w.dates.trade_deadline.date.add(-1) { w.dates.trade_deadline.date } else { d.add(2) };
    let picks: Vec<Pick> = package.iter().map(|&(year, round, orig)| Pick { year, round, orig, comp: false }).collect();
    let words: Vec<String> = picks.iter().map(|p| pick_words(w, p)).collect();
    let name = w.people[i].name.clone();
    let text = format!(
        "The {} call about {name}: they offer {}. Answer by {}.",
        w.nfl[buyer].name,
        words.join(" and "),
        until.pretty()
    );
    w.front_office.trades.calls.push(Call { date: d, until, buyer, player: id, picks, answered: false });
    super::tell_about(w, d, text, true, &[i]);
}

/// Your answer to a call: yes makes the trade as offered.
pub fn answer_call(w: &mut World, k: usize, yes: bool) -> Result<String, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let call = w.front_office.trades.calls.get(k).cloned().ok_or("No such call.")?;
    if call.answered {
        return Err("You answered that call already.".into());
    }
    w.front_office.trades.calls[k].answered = true;
    if !yes {
        return Ok(format!("You turned the {} down.", w.nfl[call.buyer].name));
    }
    let p = Proposal {
        other: call.buyer,
        give: Side { players: vec![call.player], picks: Vec::new() },
        get: Side { players: Vec::new(), picks: call.picks.clone() },
        conditional: None,
    };
    let at = super::indices(w, &ids(&p));
    check(w, me, &p, &at)?;
    if !call.picks.iter().all(|pk| owns(w, call.buyer, pk)) {
        return Err("They no longer have those picks.".into());
    }
    execute(w, me, &p);
    Ok(w.front_office.trades.made.last().map(|x| x.1.clone()).unwrap_or_default())
}

/// The desk's day: calls that ran out, conditional picks settled once
/// their season is over.
pub(crate) fn day(w: &mut World, d: Date) {
    for c in w.front_office.trades.calls.iter_mut() {
        if !c.answered && d > c.until {
            c.answered = true;
        }
    }
    w.front_office.trades.calls.retain(|c| c.date.days_to(d) < 120);
    let year = w.year;
    let due: Vec<Conditional> =
        w.front_office.trades.conditionals.iter().copied().filter(|c| c.season < year).collect();
    w.front_office.trades.conditionals.retain(|c| c.season >= year);
    for c in due {
        let met = super::index_of(w, c.player)
            .and_then(|i| w.people[i].pro.as_ref())
            .and_then(|p| p.seasons.iter().find(|s| s.year == c.season))
            .is_some_and(|s| match c.cond {
                Condition::Games(n) => s.games >= n,
                Condition::Starts(n) => s.starts >= n,
                Condition::ProBowl => s.pro_bowl,
            });
        if !met {
            continue;
        }
        // Met: the pick moves up a round (the giver's own pick in that
        // round, or one it holds), and the lower one comes back.
        let up = Pick { round: c.pick.round - 1, ..c.pick };
        if owns(w, c.to, &c.pick) && owns(w, c.from, &up) {
            move_pick(w, &up, c.to);
            move_pick(w, &c.pick, c.from);
        } else if owns(w, c.to, &c.pick) {
            trades::give_pick(w, c.from, c.to, up.year, up.round);
        }
        let text = format!(
            "A condition was met: the {} {} {}-round pick to the {} becomes a {}-round pick.",
            pro::possessive(w.nfl[c.from].name),
            c.pick.year,
            round_word(c.pick.round),
            w.nfl[c.to].name,
            round_word(up.round)
        );
        super::tell(w, d, text, false);
    }
}
