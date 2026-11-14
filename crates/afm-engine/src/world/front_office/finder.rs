//! The trade finder (TF1-TF19): shopping one of your players over days.
//! You set up the shop (quiet to the clubs you pick, or open to the
//! league; your asking price; a pay-down; leave to talk a new deal), the
//! clubs answer over a day or two with full offers that expire, you
//! counter, play one bidder against another (truly or with a bluff), and an
//! accepted offer closes through the owners, his new deal and the
//! physical. Moving a bad contract and draft night run on the same rules.
//! Every price comes from the other GM's own value (`trades::player_points`)
//! and the shared offer builder (`pro::bargain`), the one the AI's trades
//! use (TF20).
//!
//! Sizes are Claude's, proposed, not confirmed: the days to answer and to
//! hold an offer, the leaks, the 60% floor, the counter's end chance, the
//! bluff's odds, the distrust and the open shop's sting.

use super::deals::{self, Pick, Proposal, Side};
use crate::calendar::{Date, Kind, Loudness};
use crate::rng::clamp;
use crate::world::World;
use crate::world::pro::bargain::{self, Exam, Limits, Package};
use crate::world::pro::market::{self, Offer};
use crate::world::pro::{self, DeadWhy, money, trades};

/// Your asking price (TF3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ask {
    /// The best offers that come.
    #[default]
    Best,
    /// A pick in this round of the next draft (or better).
    Round(u32),
    /// A starter back and a pick: his worth to you.
    StarterAndPick,
}

/// Where a club stands in your shop.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum State {
    /// Still thinking: the answer comes by `due`.
    #[default]
    Waiting,
    /// An offer on the table until `until`.
    Offer,
    /// Not interested, with the reason.
    Passed(String),
    /// You let it lapse, or it expired.
    Lapsed,
    /// He dropped out (a rival's price beyond his top).
    Dropped,
    /// Talks ended (he walked, or you did).
    Ended(String),
    /// His offer won: it's closing, or done.
    Won,
}

/// One club in your shop.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Bid {
    pub club: usize,
    pub state: State,
    pub due: Date,
    pub until: Date,
    /// His top, his aim and his bid now, on his own chart (hidden: the
    /// screens show only the package and your staff's range).
    pub top: f64,
    pub aim: f64,
    pub bid: f64,
    pub package: Option<Package>,
    /// The offer hangs on a new deal with him (TF11).
    pub ext: Option<Offer>,
    /// 0-1: how much more haggling he takes.
    pub patience: f64,
    pub counters: u32,
}

/// A line in the talks.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub date: Date,
    pub club: Option<usize>,
    pub text: String,
    /// Your own line (else theirs, or news).
    pub you: bool,
}

/// One step of the closing.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Step {
    pub what: String,
    pub due: Date,
    /// None: waiting; Some(true) done; Some(false) failed.
    pub done: Option<bool>,
}

/// An accepted offer on its way to the league office (TF11, TF15, TF16).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Closing {
    pub club: usize,
    pub package: Package,
    pub ext: Option<Offer>,
    /// The owners' sign-off (club, step), his new deal, the physical.
    pub owners: Vec<(usize, Step)>,
    pub deal: Option<Step>,
    pub physical: Step,
    /// The physical found something new: what the buyer offers now.
    pub revised: Option<Package>,
    /// How it ended: done, or why not.
    pub result: Option<String>,
}

/// The shop around one of your players.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Shop {
    pub player: u32,
    pub opened: Date,
    /// Open to the league (else quiet, to the clubs you picked).
    pub open: bool,
    pub ask: Ask,
    pub paydown: f64,
    /// Leave for the clubs to talk a new deal with his agent.
    pub agent: bool,
    pub bids: Vec<Bid>,
    pub log: Vec<Line>,
    pub closing: Option<Closing>,
    pub closed: bool,
    /// Word got out (quiet shops leak now and then).
    pub leaked: bool,
}

fn player(w: &World) -> Result<(usize, usize, Shop), String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let s = w.front_office.trades.shop.clone().ok_or("You aren't shopping anyone.")?;
    let i = super::index_of(w, s.player).ok_or("He's gone.")?;
    Ok((me, i, s))
}

fn line(s: &mut Shop, date: Date, club: Option<usize>, you: bool, text: String) {
    s.log.push(Line { date, club, text, you });
}

/// Your asking price in points on your chart (TF3).
pub fn ask_points(w: &World, me: usize, i: usize, ask: Ask) -> f64 {
    let year = deals::first_draft(w);
    match ask {
        Ask::Best => 0.0,
        Ask::Round(r) => {
            let teams = crate::world::franchise::active_teams_in(w, w.year);
            let n = teams.len().max(1) as f64;
            teams.iter().map(|&o| trades::pick_value(w, me, year, r, o)).sum::<f64>() / n
        }
        Ask::StarterAndPick => trades::player_points(w, me, i).max(0.0),
    }
}

/// This year's pay a pay-down covers (TF12): the share of what is still
/// to come in base salary.
pub fn paydown_amount(w: &World, i: usize, share: f64) -> f64 {
    let year = w.year;
    let left = 1.0 - trades::paid_share(w, w.today);
    w.people[i]
        .pro
        .as_ref()
        .and_then(|c| c.contract.as_ref())
        .and_then(|k| k.year(year))
        .map_or(0.0, |y| share * y.base * left)
}

/// What he is worth to a club on a new deal it would sign him to (TF11):
/// this season on his deal, then the new years at the new money.
fn ext_points(w: &World, t: usize, i: usize, o: &Offer) -> f64 {
    let year = w.year;
    let Some(k) = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()) else { return 0.0 };
    let (_, apy) = market::worth(w, t, i);
    let pos = w.people[i].player.position;
    let age = w.people[i].player.age;
    let now = market::decline(pos, age);
    let left = 1.0 - trades::paid_share(w, w.today);
    let race = bargain::race(w, t);
    let rebuild = w.clubs[t].office.mode == pro::office::Mode::Rebuild;
    let mut pts = 0.0;
    let this = k.year(year).map_or(0.0, |x| x.base + x.roster_bonus + x.workout + x.per_game);
    let s0 = (apy - this) * left;
    pts += 10_000.0 * s0 * if s0 > 0.0 { if rebuild && age >= 27 { 0.35 } else { 0.5 + race } } else { 1.0 }
        / money::price_cap(year);
    for n in 1..=o.years {
        let y = year + n as i32;
        let s = apy * market::decline(pos, age + n) / now - o.apy;
        let used = if s > 0.0 && rebuild && age + n >= 27 { 0.35 } else { 1.0 };
        pts += 10_000.0 * s * used / money::price_cap(y);
    }
    if pts > 0.0 {
        pts *= 1.0 - bargain::health_cut(bargain::doctors(w, t, i));
    }
    pts
}

/// The new deal a club would offer him to make the trade (TF11): only for
/// a man in the last year or two of his deal, two years or more.
fn ext_offer(w: &World, t: usize, i: usize) -> Option<Offer> {
    let k = w.people[i].pro.as_ref()?.contract.as_ref()?;
    if k.last_year() > w.year + 1 {
        return None;
    }
    let id = market::ideal(w, i, w.today);
    let mut o = market::make_offer(w, t, i, &id, 40, 0.0, w.year + 1)?;
    o.years = o.years.max(2);
    o.apy = o.apy.min(market::worth(w, t, i).1);
    Some(o)
}

/// A club's top price for him (hidden), with a pay-down counted and, with
/// your leave, on the new deal it would sign him to.
fn top_for(w: &World, me: usize, t: usize, i: usize, paydown: f64, agent: bool) -> (f64, Option<Offer>) {
    let year = w.year;
    let relief = 10_000.0 * paydown_amount(w, i, paydown) / money::price_cap(year);
    let plain = trades::player_points(w, t, i) - deals::margin(w, t, me) + relief;
    if agent && let Some(o) = ext_offer(w, t, i) {
        let on_deal = ext_points(w, t, i, &o) - deals::margin(w, t, me) + relief;
        if on_deal > plain + 25.0 {
            return (on_deal, Some(o));
        }
    }
    (plain, None)
}

/// Why a club won't call back, if it won't (TF2, TF3, TF4).
fn passes(w: &World, me: usize, t: usize, i: usize, top: f64, ask: f64, paydown: f64) -> Option<String> {
    let year = w.year;
    if deals::waives_not(w, i, me, t) {
        return Some("He won't waive his no-trade clause for them".into());
    }
    let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
    let owed =
        k.map_or(0.0, |k| trades::left_to_pay(k, year, trades::paid_share(w, w.today))) - paydown_amount(w, i, paydown);
    if money::capped(year) && owed > pro::space(w, t, year) {
        let half = owed + paydown_amount(w, i, paydown) - paydown_amount(w, i, 0.5);
        return Some(if paydown < 0.5 && half <= pro::space(w, t, year) {
            "Short of room: only with a pay-down".into()
        } else {
            "No cap room for him".into()
        });
    }
    if top <= 30.0 {
        return Some("Won't buy".into());
    }
    if top < 0.6 * ask {
        return Some("Below your ask: won't call back".into());
    }
    None
}

/// Your staff's read of one club for the setup (TF4): its need, cap room,
/// window and a range for what he'd fetch there. The club's top stays
/// hidden: the range is your staff's guess at where its offers land.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Read {
    pub club: usize,
    pub need: f64,
    pub room: f64,
    pub fits: bool,
    pub race: f64,
    pub mode: &'static str,
    pub low: f64,
    pub high: f64,
    pub pass: Option<String>,
}

pub fn reads(w: &World, id: u32, ask: Ask, paydown: f64, agent: bool) -> Result<Vec<Read>, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(me) {
        return Err("He isn't yours to trade.".into());
    }
    let year = w.year;
    let ask_pts = ask_points(w, me, i, ask);
    let mut out = Vec::new();
    for t in crate::world::franchise::active_teams_in(w, year) {
        if t == me {
            continue;
        }
        let (top, _) = top_for(w, me, t, i, paydown, agent);
        let pass = passes(w, me, t, i, top, ask_pts, paydown);
        let aim = bargain::aim(w, t, me, i, top.max(0.0));
        let open = bargain::opener(w, t, aim);
        let n1 = super::key_n(w, 0x7F01 ^ t as u64, id as u64 ^ year as u64);
        let n2 = super::key_n(w, 0x7F02 ^ t as u64, id as u64 ^ year as u64);
        let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref());
        let owed = k.map_or(0.0, |k| trades::left_to_pay(k, year, trades::paid_share(w, w.today)))
            - paydown_amount(w, i, paydown);
        let room = pro::space(w, t, year);
        out.push(Read {
            club: t,
            need: bargain::upgrade(w, t, i),
            room,
            fits: !money::capped(year) || owed <= room,
            race: bargain::race(w, t),
            mode: match w.clubs[t].office.mode {
                pro::office::Mode::Contend => "Contending",
                pro::office::Mode::Rebuild => "Rebuilding",
                _ => "In the middle",
            },
            low: (open * (1.0 + 0.12 * n1)).min(aim * (1.0 + 0.12 * n2)).max(0.0),
            high: (open * (1.0 + 0.12 * n1)).max(aim * (1.0 + 0.12 * n2)).max(0.0),
            pass,
        });
    }
    out.sort_by(|a, b| b.high.total_cmp(&a.high));
    Ok(out)
}

/// What leverage the league knows about (TF9), for the setup's note.
pub fn leverage(w: &World, id: u32) -> Vec<&'static str> {
    let Some(me) = super::club(w) else { return vec![] };
    super::index_of(w, id).map_or(vec![], |i| bargain::leverage(w, me, i).1)
}

/// Opens the shop (TF1, TF2): the calls go out; each club answers in a day
/// or two (within hours on deadline day). A quiet call leaks one time in
/// ten; an open shop is news, and he hears it (TF19).
pub fn start(
    w: &mut World,
    id: u32,
    open: bool,
    ask: Ask,
    clubs: Vec<usize>,
    paydown: f64,
    agent: bool,
) -> Result<(), String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if !deals::window_open(w) {
        return Err("Trades are closed until the new league year.".into());
    }
    if w.front_office.trades.shop.as_ref().is_some_and(|s| !s.closed) {
        return Err("Close the shop you have open first.".into());
    }
    let c = w.people[i].pro.as_ref().ok_or("Not a pro.")?;
    if c.team != Some(me) || c.contract.as_ref().is_none_or(|k| k.team != me) {
        return Err("He isn't yours to trade.".into());
    }
    if c.out_weeks > 0 {
        return Err("He's hurt: no club takes on an injured player.".into());
    }
    if !open && clubs.is_empty() {
        return Err("Pick the clubs to call.".into());
    }
    let paydown = clamp(paydown, 0.0, 0.5);
    let d = w.today;
    let deadline = d >= w.dates.trade_deadline.date;
    let year = w.year;
    let ask_pts = ask_points(w, me, i, ask);
    let targets: Vec<usize> = if open {
        crate::world::franchise::active_teams_in(w, year).into_iter().filter(|&t| t != me).collect()
    } else {
        clubs.into_iter().filter(|&t| t != me && t < w.nfl.len()).collect()
    };
    let mut s = Shop {
        player: id,
        opened: d,
        open,
        ask,
        paydown,
        agent,
        bids: vec![],
        log: vec![],
        closing: None,
        closed: false,
        leaked: false,
    };
    let name = w.people[i].name.clone();
    line(
        &mut s,
        d,
        None,
        true,
        if open {
            format!("You put word out that {name} is available.")
        } else {
            format!("Your calls go out about {name} to {} clubs.", targets.len())
        },
    );
    for t in targets {
        let (top, ext) = top_for(w, me, t, i, paydown, agent);
        let mut b = Bid { club: t, top, ext, patience: 1.0, ..Default::default() };
        if let Some(why) = passes(w, me, t, i, top, ask_pts, paydown) {
            b.state = State::Passed(why);
        } else {
            // An open shop tells every buyer there's a market: each keeps
            // less of the gap for himself.
            let k = if open { 0.8 } else { 1.0 };
            let aim = bargain::aim(w, t, me, i, top);
            b.aim = top - (top - aim) * k;
            b.bid = bargain::opener(w, t, b.aim);
            b.due = if deadline { d } else { d.add(1 + (super::key(w, 0x7F10 ^ t as u64, id as u64) < 0.5) as i32) };
        }
        if !open && super::key(w, 0x7F11 ^ t as u64, id as u64 ^ d.0 as u64) < 0.1 {
            s.leaked = true;
        }
        s.bids.push(b);
    }
    let who = pro::describe(w, i);
    let club = w.nfl[me].name;
    if open || s.leaked {
        let loyal = market::goals(w, i).loyalty;
        let hit = if open { 0.15 + 0.3 * loyal } else { 0.1 + 0.2 * loyal };
        if let Some(c) = w.people[i].pro.as_mut() {
            c.trust = clamp(c.trust - hit, 0.0, 1.0);
        }
        let age = w.people[i].row_age();
        w.people[i].note(format!("Learned at {age} that the {club} were shopping him."));
        let text = if open {
            format!("The {club} are taking calls on {who}.")
        } else {
            format!("Reports: the {club} have been quietly shopping {who}.")
        };
        market::post(w, d, Some(me), Some(i), Loudness::Medium, Kind::News, text.clone());
        line(&mut s, d, None, false, text);
    }
    w.front_office.trades.shop = Some(s);
    super::note(w, format!("You started shopping {name}."), Some(id));
    if deadline {
        day(w, d);
    }
    Ok(())
}

fn lim() -> Limits {
    Limits { back: true, draft_only: None, cond: true, swap: true }
}

/// The shop's day: answers come in, offers lapse, the closing moves on.
pub(crate) fn day(w: &mut World, d: Date) {
    let Ok((me, i, mut s)) = player(w) else { return };
    if s.closed {
        return;
    }
    let deadline = w.dates.trade_deadline.date;
    if d > deadline && s.closing.is_none() {
        for b in s.bids.iter_mut().filter(|b| matches!(b.state, State::Offer | State::Waiting)) {
            b.state = State::Lapsed;
        }
        s.closed = true;
        line(&mut s, d, None, false, "The trade deadline passed with the shop open.".into());
        w.front_office.trades.shop = Some(s);
        return;
    }
    let mut news = Vec::new();
    for k in 0..s.bids.len() {
        let b = s.bids[k].clone();
        match b.state {
            State::Waiting if b.due <= d => {
                let t = b.club;
                match bargain::build(w, t, me, i, b.bid, lim()) {
                    Some(pk) => {
                        let hold = if d >= deadline {
                            0
                        } else {
                            2 + (super::key(w, 0x7F12 ^ t as u64, d.0 as u64) < 0.5) as i32
                        };
                        let bb = &mut s.bids[k];
                        bb.until = bargain::days_on(w, d, hold);
                        bb.package = Some(pk.clone());
                        bb.state = State::Offer;
                        let deal = if b.ext.is_some() { ", if he signs a new deal with them" } else { "" };
                        news.push((t, format!("The {} offer {}{deal}.", w.nfl[t].name, package_words(w, &pk, t))));
                    }
                    None => {
                        s.bids[k].state = State::Passed("Nothing that fits".into());
                        news.push((t, format!("The {} pass.", w.nfl[t].name)));
                    }
                }
            }
            State::Offer if b.until < d => {
                s.bids[k].state = State::Lapsed;
                news.push((b.club, format!("The {} offer lapsed.", w.nfl[b.club].name)));
            }
            _ => {}
        }
    }
    let any = !news.is_empty();
    for (t, text) in news {
        line(&mut s, d, Some(t), false, text);
    }
    w.front_office.trades.shop = Some(s);
    if any {
        let name = w.people[i].name.clone();
        super::tell_about(w, d, format!("Trade finder: news in the shop for {name}."), false, &[i]);
    }
    closing_day(w, d);
}

/// Picks and players in words.
/// `from` is the club giving them: its own picks read "its".
pub fn package_words(w: &World, pk: &Package, from: usize) -> String {
    let mut v: Vec<String> = pk
        .back
        .iter()
        .filter_map(|&id| super::index_of(w, id))
        .map(|j| format!("{} ({})", w.people[j].name, w.people[j].player.position.name()))
        .collect();
    for p in &pk.picks {
        let c =
            pk.cond.filter(|c| c.0 == *p).map_or(String::new(), |(_, c)| format!(" (a round higher {})", c.words()));
        let pick = if p.orig == from && !p.comp {
            format!("their {} {}-round pick", p.year, deals::round_word(p.round))
        } else {
            deals::pick_words(w, p)
        };
        v.push(format!("{pick}{c}"));
    }
    if let Some((_, theirs)) = pk.swap {
        v.push(format!("a swap of {} {}-round picks", theirs.year, deals::round_word(theirs.round)));
    }
    match v.len() {
        0 => "nothing".into(),
        1 => v[0].clone(),
        _ => {
            let last = v.pop().unwrap();
            format!("{} and {last}", v.join(", "))
        }
    }
}

fn bid_at(s: &Shop, club: usize) -> Result<usize, String> {
    s.bids.iter().position(|b| b.club == club).ok_or_else(|| "That club isn't in your shop.".into())
}

/// You counter (TF7): with the picks you ask for from his list, or for
/// more. Within his aim he takes it; past it, each counter can end the
/// talks (10-35%, by his patience and the gap), else he moves up.
pub fn counter(w: &mut World, club: usize, picks: Vec<Pick>) -> Result<String, String> {
    let (me, i, mut s) = player(w)?;
    let k = bid_at(&s, club)?;
    let b = s.bids[k].clone();
    if b.state != State::Offer {
        return Err("There's no offer from them to counter.".into());
    }
    let d = w.today;
    let theirs = deals::picks_of(w, club);
    if picks.iter().any(|p| !theirs.contains(p)) {
        return Err("They don't own one of those picks.".into());
    }
    let asked = if picks.is_empty() {
        b.bid * 1.2
    } else {
        picks.iter().map(|p| deals::pick_points(w, club, p)).sum::<f64>()
            + b.package.as_ref().map_or(0.0, |pk| {
                pk.back
                    .iter()
                    .filter_map(|&id| super::index_of(w, id))
                    .map(|j| trades::player_points(w, club, j).max(0.0))
                    .sum()
            })
    };
    let ask_words = if picks.is_empty() {
        "more".to_string()
    } else {
        picks.iter().map(|p| deals::pick_words(w, p)).collect::<Vec<_>>().join(" and ")
    };
    line(&mut s, d, Some(club), true, format!("You ask the {} for {ask_words}.", w.nfl[club].name));
    let haggle = bargain::style(w, club).haggle;
    let reply;
    if asked <= b.aim {
        let mut pk = b.package.clone().unwrap_or_default();
        if !picks.is_empty() {
            pk.picks = picks.clone();
            pk.cond = None;
            pk.swap = None;
            pk.cost = asked;
            pk.worth = picks.iter().map(|p| deals::pick_points(w, me, p)).sum::<f64>()
                + pk.back
                    .iter()
                    .filter_map(|&id| super::index_of(w, id))
                    .map(|j| trades::player_points(w, me, j))
                    .sum::<f64>();
        } else if let Some(p) = bargain::build(w, club, me, i, asked, lim()) {
            pk = p;
        }
        reply = format!("\"We can do that: {}.\"", package_words(w, &pk, club));
        let bb = &mut s.bids[k];
        bb.bid = asked;
        bb.package = Some(pk);
        bb.counters += 1;
    } else {
        let gap = clamp((asked - b.aim) / b.aim.max(1.0), 0.0, 1.0);
        let end = 0.10 + 0.25 * clamp(0.5 * haggle + 0.5 * gap, 0.0, 1.0);
        let roll = super::key(w, 0x7F20 ^ club as u64, (b.counters as u64) << 32 | d.0 as u64);
        if roll < end || b.patience <= 0.0 {
            s.bids[k].state = State::Ended("He ended the talks".into());
            reply = "\"We're too far apart. We'll pass.\"".into();
        } else {
            let bid = b.bid + (b.aim - b.bid) * 0.6;
            let pk = bargain::build(w, club, me, i, bid, lim()).or(b.package.clone());
            reply = format!(
                "\"The best we can do is {}.\"",
                pk.as_ref().map_or("what we said".into(), |p| package_words(w, p, club))
            );
            let bb = &mut s.bids[k];
            bb.bid = bid;
            bb.package = pk;
            bb.counters += 1;
            bb.patience = (bb.patience - 0.15 - 0.2 * haggle).max(0.0);
            bb.until = bargain::days_on(w, d, 1).max(bb.until);
        }
    }
    line(&mut s, d, Some(club), false, reply.clone());
    w.front_office.trades.shop = Some(s);
    Ok(reply)
}

/// How likely a GM believes a rival offers more (TF8): a rival with a
/// need at the spot, the room and a reason to win now is believable; a
/// sharp GM doubts more.
pub fn believable(w: &World, club: usize, rival: usize, i: usize) -> f64 {
    let year = w.year;
    let need = clamp(bargain::upgrade(w, rival, i) / 8.0, 0.0, 1.0);
    let owed = w.people[i]
        .pro
        .as_ref()
        .and_then(|c| c.contract.as_ref())
        .map_or(0.0, |k| trades::left_to_pay(k, year, trades::paid_share(w, w.today)));
    let cap = if !money::capped(year) || owed <= pro::space(w, rival, year) { 1.0 } else { 0.25 };
    let window = if w.clubs[rival].office.mode == pro::office::Mode::Rebuild { 0.3 } else { 1.0 };
    let skill = w.clubs[club].office.gm.skill;
    clamp(0.1 + 0.55 * need * cap * window - 0.15 * (skill - 10.0) / 10.0, 0.02, 0.9)
}

/// You tell a bidder a rival offers more (TF8). The truth makes him raise
/// or drop out; a bluff he believes does the same; a bluff he catches
/// costs his trust for a year and his patience now.
pub fn rival(w: &mut World, club: usize, rival: usize, bluff: bool) -> Result<String, String> {
    let (me, i, mut s) = player(w)?;
    let k = bid_at(&s, club)?;
    let b = s.bids[k].clone();
    if b.state != State::Offer {
        return Err("There's no offer from them to play against.".into());
    }
    if rival == club || rival == me || rival >= w.nfl.len() {
        return Err("Pick another club.".into());
    }
    let d = w.today;
    let true_offer =
        s.bids.iter().find(|r| r.club == rival && r.state == State::Offer).map(|r| r.bid).filter(|&x| x >= b.bid);
    if !bluff && true_offer.is_none() {
        return Err("They don't offer more. That would be a bluff.".into());
    }
    let claim = true_offer.filter(|_| !bluff).unwrap_or(b.bid * 1.2);
    line(
        &mut s,
        d,
        Some(club),
        true,
        format!("You tell the {} the {} offer more.", w.nfl[club].name, w.nfl[rival].name),
    );
    let believed =
        !bluff || super::key(w, 0x7F30 ^ club as u64, rival as u64 ^ (d.0 as u64) << 8) < believable(w, club, rival, i);
    let reply = if !believed {
        w.front_office.trades.distrust.retain(|x| x.0 != club);
        w.front_office.trades.distrust.push((club, d.add(365)));
        let bb = &mut s.bids[k];
        bb.patience -= 0.3;
        if bb.patience <= 0.0 {
            bb.state = State::Ended("He caught your bluff".into());
            "\"I don't believe you. We're done here.\"".to_string()
        } else {
            "\"I don't buy that. Our offer stands.\"".to_string()
        }
    } else if claim > b.top {
        s.bids[k].state = State::Dropped;
        "\"Then take theirs. We won't go that high.\"".to_string()
    } else {
        let aim = b.aim.max((claim * 1.03).min(b.top));
        let pk = bargain::build(w, club, me, i, aim, lim()).or(b.package.clone());
        let words = pk.as_ref().map_or("more".to_string(), |p| package_words(w, p, club));
        let bb = &mut s.bids[k];
        bb.aim = aim;
        bb.bid = aim;
        bb.package = pk;
        bb.until = bargain::days_on(w, d, 1).max(bb.until);
        format!("\"All right. We'll go to {words}.\"")
    };
    line(&mut s, d, Some(club), false, reply.clone());
    w.front_office.trades.shop = Some(s);
    Ok(reply)
}

/// You let an offer lapse, walk away from a club, or close the shop.
pub fn lapse(w: &mut World, club: usize, walk: bool) -> Result<(), String> {
    let (_, _, mut s) = player(w)?;
    let k = bid_at(&s, club)?;
    let d = w.today;
    s.bids[k].state = if walk { State::Ended("You walked away".into()) } else { State::Lapsed };
    let verb = if walk { "walk away from" } else { "let lapse" };
    line(&mut s, d, Some(club), true, format!("You {verb} the {} offer.", w.nfl[club].name));
    w.front_office.trades.shop = Some(s);
    Ok(())
}

pub fn close(w: &mut World) -> Result<(), String> {
    let (_, _, mut s) = player(w)?;
    if s.closing.as_ref().is_some_and(|c| c.result.is_none()) {
        return Err("A deal is closing: wait for it, or walk away from it first.".into());
    }
    let d = w.today;
    for b in s.bids.iter_mut().filter(|b| matches!(b.state, State::Offer | State::Waiting)) {
        b.state = State::Lapsed;
    }
    s.closed = true;
    line(&mut s, d, None, true, "You closed the shop.".into());
    w.front_office.trades.shop = Some(s);
    Ok(())
}

/// You accept an offer (TF16, TF11, TF15): the owners sign off on a
/// blockbuster, his agent and the club agree a new deal if it hangs on
/// one, and the buyer's doctors examine him. Each takes a day; on
/// deadline day they all happen at once.
pub fn accept(w: &mut World, club: usize) -> Result<(), String> {
    let (me, i, mut s) = player(w)?;
    if s.closing.as_ref().is_some_and(|c| c.result.is_none()) {
        return Err("A deal is already closing.".into());
    }
    let k = bid_at(&s, club)?;
    let b = s.bids[k].clone();
    let Some(pk) = b.package.clone().filter(|_| b.state == State::Offer) else {
        return Err("There's no offer from them to accept.".into());
    };
    let d = w.today;
    let deadline = d >= w.dates.trade_deadline.date;
    let next = if deadline { d } else { d.add(1) };
    let mut owners = Vec::new();
    if bargain::blockbuster(w, i, &pk.picks) {
        for t in [club, me] {
            if w.clubs[t].office.power == pro::office::Power::Owner {
                owners
                    .push((t, Step { what: format!("The {} owner signs off", w.nfl[t].name), due: next, done: None }));
            }
        }
    }
    let deal = b.ext.map(|_| Step { what: "His new deal with them".into(), due: next, done: None });
    let physical = Step { what: "The physical".into(), due: if deadline { d } else { d.add(1).max(next) }, done: None };
    s.bids[k].state = State::Won;
    line(&mut s, d, Some(club), true, format!("You accept the {} offer.", w.nfl[club].name));
    s.closing = Some(Closing { club, package: pk, ext: b.ext, owners, deal, physical, revised: None, result: None });
    w.front_office.trades.shop = Some(s);
    closing_day(w, d);
    Ok(())
}

fn closing_day(w: &mut World, d: Date) {
    let Ok((me, i, mut s)) = player(w) else { return };
    let Some(mut c) = s.closing.clone().filter(|c| c.result.is_none() && c.revised.is_none()) else { return };
    let mut fail: Option<String> = None;
    for (t, st) in c.owners.iter_mut() {
        if st.done.is_none() && st.due <= d {
            let ok = bargain::owner_yes(w, *t, *t != me, w.people[i].player.id as u64 ^ c.club as u64).unwrap_or(true);
            st.done = Some(ok);
            if !ok {
                fail = Some(format!("The {} owner said no.", w.nfl[*t].name));
            }
        }
    }
    if let (Some(st), Some(o)) = (c.deal.as_mut(), c.ext) {
        if st.done.is_none() && st.due <= d {
            let id = market::ideal(w, i, d);
            let ok = market::score(w, i, &o, &id, Some(c.club)) >= 0.82;
            st.done = Some(ok);
            if !ok {
                fail = fail.or(Some("He and the club couldn't agree a new deal.".into()));
            }
        }
    }
    let ready = c.owners.iter().all(|x| x.1.done == Some(true)) && c.deal.as_ref().is_none_or(|x| x.done == Some(true));
    if fail.is_none() && ready && c.physical.done.is_none() && c.physical.due <= d {
        match bargain::exam(w, c.club, i, d.0 as u64 ^ 0x7F40).0 {
            Exam::Passed => c.physical.done = Some(true),
            Exam::Failed => {
                c.physical.done = Some(false);
                fail = Some("He failed the physical: the trade is off.".into());
            }
            Exam::Revised => {
                c.physical.done = Some(false);
                let mut pk = c.package.clone();
                if let Some(k) = (0..pk.picks.len()).min_by(|&a, &b| {
                    deals::pick_points(w, c.club, &pk.picks[a]).total_cmp(&deals::pick_points(w, c.club, &pk.picks[b]))
                }) {
                    let p = pk.picks.remove(k);
                    pk.cost -= deals::pick_points(w, c.club, &p);
                    pk.worth -= deals::pick_points(w, me, &p);
                    if pk.cond.is_some_and(|x| x.0 == p) {
                        pk.cond = None;
                    }
                }
                let words = package_words(w, &pk, c.club);
                line(&mut s, d, Some(c.club), false, format!("Their doctors found something. They now offer {words}."));
                c.revised = Some(pk);
                s.closing = Some(c);
                w.front_office.trades.shop = Some(s);
                let name = w.people[i].name.clone();
                super::tell_about(
                    w,
                    d,
                    format!(
                        "{name}'s physical turned something up: the trade waits on your answer to their new terms."
                    ),
                    true,
                    &[i],
                );
                return;
            }
        }
    }
    if let Some(why) = fail {
        c.result = Some(why.clone());
        if let Some(b) = s.bids.iter_mut().find(|b| b.club == c.club) {
            b.state = State::Ended(why.clone());
        }
        line(&mut s, d, Some(c.club), false, why.clone());
        s.closing = Some(c);
        w.front_office.trades.shop = Some(s);
        let name = w.people[i].name.clone();
        super::tell_about(w, d, format!("The trade of {name} fell through: {why}"), false, &[i]);
        return;
    }
    let done = c.physical.done == Some(true);
    s.closing = Some(c);
    w.front_office.trades.shop = Some(s);
    if done {
        complete(w, d);
    }
}

/// Your answer to the buyer's terms after the physical.
pub fn revised(w: &mut World, yes: bool) -> Result<(), String> {
    let (_, _, mut s) = player(w)?;
    let mut c = s.closing.clone().ok_or("No deal is closing.")?;
    let pk = c.revised.take().ok_or("No new terms to answer.")?;
    let d = w.today;
    if yes {
        c.package = pk;
        c.physical.done = Some(true);
        line(&mut s, d, Some(c.club), true, "You take the new terms.".into());
        s.closing = Some(c);
        w.front_office.trades.shop = Some(s);
        complete(w, d);
    } else {
        c.result = Some("You walked away after the physical.".into());
        if let Some(b) = s.bids.iter_mut().find(|b| b.club == c.club) {
            b.state = State::Ended("You walked after the physical".into());
        }
        line(&mut s, d, Some(c.club), true, "You walk away from the new terms.".into());
        s.closing = Some(c);
        w.front_office.trades.shop = Some(s);
    }
    Ok(())
}

/// His base this year less a pay-down, for the league's check; gives back
/// what to restore.
fn lower_base(w: &mut World, i: usize, amount: f64) -> Option<(f64, f64)> {
    let year = w.year;
    let y = w.people[i].pro.as_mut()?.contract.as_mut()?.years.iter_mut().find(|y| y.year == year && !y.void)?;
    let was = (y.base, y.guaranteed);
    y.base -= amount.min(y.base);
    Some(was)
}

fn restore_base(w: &mut World, i: usize, was: Option<(f64, f64)>) {
    let year = w.year;
    if let Some((b, g)) = was
        && let Some(y) = w.people[i]
            .pro
            .as_mut()
            .and_then(|c| c.contract.as_mut())
            .and_then(|k| k.years.iter_mut().find(|y| y.year == year && !y.void))
    {
        y.base = b;
        y.guaranteed = g;
    }
}

/// Turns this year's salary into a bonus you pay (TF12): his base drops by
/// the amount, which stays on your cap as dead money and is your cash.
fn pay_down(w: &mut World, me: usize, i: usize, amount: f64) {
    if amount <= 0.0 {
        return;
    }
    let year = w.year;
    let id = w.people[i].player.id;
    let Some(c) = w.people[i].pro.as_mut() else { return };
    let Some(y) = c.contract.as_mut().and_then(|k| k.years.iter_mut().find(|y| y.year == year && !y.void)) else {
        return;
    };
    let cut = amount.min(y.base);
    y.base -= cut;
    y.guaranteed = y.guaranteed.min(y.base);
    c.earnings += cut;
    if let Some(s) = c.seasons.last_mut().filter(|s| s.year == year) {
        s.pay += cut;
    }
    w.clubs[me].add_dead(year, cut, id, DeadWhy::PaidDown);
    match w.clubs[me].cash.last_mut() {
        Some(x) if x.0 == year => x.1 += cut,
        _ => w.clubs[me].cash.push((year, cut)),
    }
}

fn proposal(me: usize, club: usize, id: u32, pk: &Package) -> Proposal {
    let mut give = Side { players: vec![id], picks: vec![] };
    let mut get = Side { players: pk.back.clone(), picks: pk.picks.clone() };
    if let Some((mine, theirs)) = pk.swap {
        give.picks.push(mine);
        get.picks.push(theirs);
    }
    let _ = me;
    Proposal { other: club, give, get, conditional: None }
}

/// The deal goes through: the pay-down, the trade, the condition on his
/// pick, his new deal with them.
fn complete(w: &mut World, d: Date) {
    let Ok((me, i, mut s)) = player(w) else { return };
    let Some(mut c) = s.closing.clone() else { return };
    let id = s.player;
    let p = proposal(me, c.club, id, &c.package);
    let at = super::indices(w, &[&p.give.players[..], &p.get.players[..]].concat());
    let amount = paydown_amount(w, i, s.paydown);
    // The league reads the deal with the pay-down done, so the buyer's cap
    // sees the lower salary.
    let base = lower_base(w, i, amount);
    let ok = deals::check(w, me, &p, &at);
    restore_base(w, i, base);
    if let Err(why) = ok {
        c.result = Some(why.clone());
        line(&mut s, d, Some(c.club), false, format!("The league office turned it down: {why}"));
        s.closing = Some(c);
        w.front_office.trades.shop = Some(s);
        return;
    }
    pay_down(w, me, i, amount);
    deals::execute(w, me, &p);
    if let Some((pk, cond)) = c.package.cond {
        w.front_office.trades.conditionals.push(deals::Conditional {
            pick: pk,
            from: c.club,
            to: me,
            player: id,
            cond,
            season: w.year,
        });
    }
    if let Some(o) = c.ext {
        market::extend(w, i, &o, d);
    }
    c.result = Some("Done".into());
    let text = format!("The trade is done: {} to the {}.", w.people[i].name, w.nfl[c.club].name);
    line(&mut s, d, Some(c.club), false, text);
    for b in s.bids.iter_mut().filter(|b| matches!(b.state, State::Offer | State::Waiting)) {
        b.state = State::Lapsed;
    }
    s.closing = Some(c);
    s.closed = true;
    w.front_office.trades.shop = Some(s);
}

// --- Moving a bad contract (TF13) -----------------------------------------------------------------

/// One way to move a contract nobody takes as it is.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Dump {
    pub club: usize,
    /// "picks", "paydown" or "swap".
    pub kind: String,
    /// What you send with him.
    pub picks: Vec<Pick>,
    pub paydown: f64,
    /// Their bad deal you take back (id).
    pub back: Option<u32>,
    /// What it costs you on your chart.
    pub cost: f64,
    /// Your dead money this year, and the cap you save this year.
    pub dead: f64,
    pub saved: f64,
}

/// What it costs to move him, club by club (TF13): the cheapest of picks
/// alone, a half pay-down and picks, or their bad deal back and picks,
/// each sorted by what it costs you.
pub fn dumps(w: &World, id: u32) -> Result<(f64, Vec<Dump>), String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(me) {
        return Err("He isn't yours to trade.".into());
    }
    let year = w.year;
    let june1 = pro::after_june1(w, w.today);
    let k = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).ok_or("He has no contract.")?;
    let hit = k.cap_hit(year);
    let kept = trades::kept_bonus(k, year, june1).0;
    let share = trades::paid_share(w, w.today);
    let paid = hit - k.year(year).map_or(0.0, |x| x.proration) - trades::left_to_pay(k, year, share);
    let mine = deals::picks_of(w, me);
    let mut out = Vec::new();
    for t in crate::world::franchise::active_teams_in(w, year) {
        if t == me || deals::waives_not(w, i, me, t) {
            continue;
        }
        let need = deals::margin(w, t, me) - trades::player_points(w, t, i);
        let mut opts: Vec<Dump> = Vec::new();
        let add = |need: f64, kind: &str, paydown: f64, back: Option<(u32, f64, f64, f64)>, out: &mut Vec<Dump>| {
            let pd = paydown_amount(w, i, paydown);
            // Their deal back leaves its bonus on their cap (b.3).
            let owed = trades::left_to_pay(k, year, share) - pd - back.map_or(0.0, |b| b.2 - b.3);
            if money::capped(year) && owed > pro::space(w, t, year) {
                return;
            }
            // Your own cap after it, as the league reads it: his pay to
            // come leaves, the bonus you keep and the pay-down stay, a deal
            // back arrives.
            let mine_after = -(trades::left_to_pay(k, year, share) + k.year(year).map_or(0.0, |x| x.proration))
                + kept
                + pd
                + back.map_or(0.0, |b| b.2);
            if money::capped(year) && mine_after > pro::cap_room(w, me, year) + 1.0 {
                return;
            }
            let (picks, cost) = if need <= 0.0 {
                (vec![], 0.0)
            } else {
                match cheapest(w, me, t, &mine, need) {
                    Some(x) => x,
                    None => return,
                }
            };
            let dead = kept + paid + pd;
            out.push(Dump {
                club: t,
                kind: kind.into(),
                picks,
                paydown,
                back: back.map(|b| b.0),
                cost: cost + back.map_or(0.0, |b| -b.1),
                dead,
                saved: hit - dead - back.map_or(0.0, |b| b.2),
            });
        };
        add(need, "picks", 0.0, None, &mut opts);
        let relief = 10_000.0 * paydown_amount(w, i, 0.5) / money::price_cap(year);
        add(need - relief, "paydown", 0.5, None, &mut opts);
        // Their worst deal you'd take back: what it saves them less what it
        // costs you.
        let mut best: Option<(u32, f64, f64, f64)> = None;
        for &j in &w.clubs[t].roster {
            if !bargain::movable(w, t, j) {
                continue;
            }
            let theirs = trades::player_points(w, t, j);
            if theirs >= 0.0 {
                continue;
            }
            let yours = trades::player_points(w, me, j);
            let gain = -theirs + yours.min(0.0);
            if best.is_none_or(|b| gain > b.3) {
                let kj = w.people[j].pro.as_ref().unwrap().contract.as_ref().unwrap();
                best = Some((w.people[j].player.id, yours, trades::left_to_pay(kj, year, share), gain));
            }
        }
        if let Some((bid, yours, his_pay, _)) = best
            && let Some(j) = super::index_of(w, bid)
        {
            let theirs = trades::player_points(w, t, j);
            let kj = w.people[j].pro.as_ref().and_then(|c| c.contract.as_ref());
            let net =
                kj.map_or(0.0, |kj| trades::kept_bonus(kj, year, june1).0 - kj.year(year).map_or(0.0, |x| x.proration));
            add(need + theirs, "swap", 0.0, Some((bid, yours, his_pay, net)), &mut opts);
        }
        if let Some(b) = opts.into_iter().min_by(|a, b| {
            (a.cost + 10_000.0 * (a.dead - kept - paid) / money::price_cap(year))
                .total_cmp(&(b.cost + 10_000.0 * (b.dead - kept - paid) / money::price_cap(year)))
        }) {
            out.push(b);
        }
    }
    out.sort_by(|a, b| a.cost.total_cmp(&b.cost));
    let worth = trades::player_points(w, me, i);
    Ok((worth, out))
}

/// Your cheapest picks (on your chart) that are worth `need` on theirs.
fn cheapest(w: &World, me: usize, t: usize, mine: &[Pick], need: f64) -> Option<(Vec<Pick>, f64)> {
    let mut v: Vec<(Pick, f64, f64)> =
        mine.iter().map(|p| (*p, deals::pick_points(w, t, p), deals::pick_points(w, me, p))).collect();
    let single = v.iter().filter(|x| x.1 >= need).min_by(|a, b| a.2.total_cmp(&b.2)).map(|x| (vec![x.0], x.2));
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut pair: Option<(Vec<Pick>, f64)> = None;
    for a in 0..v.len().min(40) {
        for b in a + 1..v.len().min(40) {
            if v[a].1 + v[b].1 >= need && pair.as_ref().is_none_or(|p| v[a].2 + v[b].2 < p.1) {
                pair = Some((vec![v[a].0, v[b].0], v[a].2 + v[b].2));
            }
        }
    }
    match (single, pair) {
        (Some(s), Some(p)) => Some(if s.1 <= p.1 { s } else { p }),
        (s, p) => s.or(p),
    }
}

/// Makes one of those offers: priced at the other club's walk-away, it
/// says yes.
pub fn dump(w: &mut World, id: u32, club: usize) -> Result<String, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let (_, list) = dumps(w, id)?;
    let o = list.into_iter().find(|x| x.club == club).ok_or("They won't take him on any terms.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let p = Proposal {
        other: club,
        give: Side { players: vec![id], picks: o.picks.clone() },
        get: Side { players: o.back.into_iter().collect(), picks: vec![] },
        conditional: None,
    };
    if p.get.players.is_empty() && p.get.picks.is_empty() {
        // A pure salary dump: a seventh-round pick swap is how the league
        // books it (real: "for a conditional seventh" or nothing at all).
        let at = super::indices(w, &p.give.players);
        if !deals::window_open(w) {
            return Err("Trades are closed until the new league year.".into());
        }
        let _ = at;
    }
    let amount = paydown_amount(w, i, o.paydown);
    if !p.get.players.is_empty() {
        let at = super::indices(w, &[&p.give.players[..], &p.get.players[..]].concat());
        let base = lower_base(w, i, amount);
        let ok = deals::check(w, me, &p, &at);
        restore_base(w, i, base);
        ok?;
    }
    pay_down(w, me, i, amount);
    if p.get.players.is_empty() && p.get.picks.is_empty() {
        let d = w.today;
        let june1 = pro::after_june1(w, d);
        for pk in &p.give.picks {
            deals::move_pick(w, pk, club);
        }
        deals::move_player(w, i, me, club, d, june1);
        let text = format!(
            "The {} trade {}{} to the {} for nothing back.",
            w.nfl[me].name,
            w.people[i].name,
            if p.give.picks.is_empty() {
                String::new()
            } else {
                format!(
                    " and {}",
                    p.give.picks.iter().map(|x| deals::pick_words(w, x)).collect::<Vec<_>>().join(" and ")
                )
            },
            w.nfl[club].name
        );
        let e =
            crate::calendar::Event::new(d, crate::calendar::Level::Nfl, Kind::Transfer, Loudness::Medium, text.clone())
                .team(me);
        w.calendar.post(e);
        w.market.year_mut(w.year).trades += 1;
        w.market.picks.trades.push(trades::Trade {
            date: d,
            buyer: club,
            seller: me,
            player: id,
            picks: vec![],
            text: text.clone(),
        });
        w.front_office.trades.made.push((d, text.clone()));
        super::note(w, text.clone(), Some(id));
        return Ok(text);
    }
    deals::execute(w, me, &p);
    Ok(w.front_office.trades.made.last().map(|x| x.1.clone()).unwrap_or_default())
}

// --- Draft night (TF17) ---------------------------------------------------------------------------

/// A club's offer of tonight's picks for one of your players.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Night {
    pub club: usize,
    /// Slots on the board (index from 0).
    pub slots: Vec<usize>,
    pub worth: f64,
    pub cost: f64,
}

/// What each club would give in this draft's picks for one of your
/// players tonight: its slots still to come, the biggest that fits its
/// price first.
pub fn night_offers(w: &World, id: u32) -> Result<Vec<Night>, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    if w.people[i].pro.as_ref().and_then(|c| c.team) != Some(me) {
        return Err("He isn't yours to trade.".into());
    }
    let l = w.scouting.live.as_ref().ok_or("The draft isn't live.")?;
    let at = l.run.next as usize - 1;
    let mut out = Vec::new();
    for t in crate::world::franchise::active_teams_in(w, w.year) {
        if t == me || !bargain::movable(w, me, i) || deals::waives_not(w, i, me, t) {
            continue;
        }
        let top = trades::player_points(w, t, i) - deals::margin(w, t, me);
        if top <= 30.0 {
            continue;
        }
        let aim = bargain::aim(w, t, me, i, top);
        let classic_t = w.clubs[t].office.gm.classic_chart;
        let classic_me = w.clubs[me].office.gm.classic_chart;
        let mut left = aim;
        let mut slots = Vec::new();
        let (mut cost, mut worth) = (0.0, 0.0);
        for s in at..l.run.board.len() {
            if l.run.board[s].0 != t || slots.len() >= 2 {
                continue;
            }
            let v = trades::chart(classic_t, s as f64 + 1.0);
            if v <= left * 1.08 {
                slots.push(s);
                left -= v;
                cost += v;
                worth += trades::chart(classic_me, s as f64 + 1.0);
            }
        }
        if !slots.is_empty() && cost >= 0.5 * aim {
            out.push(Night { club: t, slots, worth, cost });
        }
    }
    out.sort_by(|a, b| b.worth.total_cmp(&a.worth));
    Ok(out)
}

/// Takes a club's picks tonight for him: he joins them, their slots are yours.
pub fn night_accept(w: &mut World, id: u32, club: usize) -> Result<String, String> {
    let me = super::club(w).ok_or("You don't run an NFL club.")?;
    let i = super::index_of(w, id).ok_or("No such player.")?;
    let o = night_offers(w, id)?.into_iter().find(|x| x.club == club).ok_or("That offer is gone.")?;
    let d = w.today;
    let june1 = pro::after_june1(w, d);
    let share = trades::paid_share(w, d);
    let owed = w.people[i]
        .pro
        .as_ref()
        .and_then(|c| c.contract.as_ref())
        .map_or(0.0, |k| trades::left_to_pay(k, w.year, share));
    if money::capped(w.year) && owed > pro::space(w, club, w.year) {
        return Err(format!("The {} don't have the cap room.", w.nfl[club].name));
    }
    deals::move_player(w, i, me, club, d, june1);
    let mut l = w.scouting.live.take().ok_or("The draft isn't live.")?;
    for &s in &o.slots {
        l.run.board[s].0 = me;
    }
    let slots: Vec<String> = o.slots.iter().map(|s| format!("pick {}", s + 1)).collect();
    let text = format!(
        "Trade: the {} send {} to the {} for {}.",
        w.nfl[me].name,
        w.people[i].name,
        w.nfl[club].name,
        slots.join(" and ")
    );
    l.feed.push(crate::world::scouting::live::Feed {
        pick: o.slots[0] as u32 + 1,
        round: l.run.board[o.slots[0]].1,
        team: me,
        player: None,
        text: text.clone(),
    });
    w.scouting.live = Some(l);
    let e = crate::calendar::Event::new(d, crate::calendar::Level::Nfl, Kind::Transfer, Loudness::Medium, text.clone())
        .team(me);
    w.calendar.post(e);
    w.market.year_mut(w.year).trades += 1;
    w.market.picks.trades.push(trades::Trade {
        date: d,
        buyer: club,
        seller: me,
        player: id,
        picks: vec![],
        text: text.clone(),
    });
    w.front_office.trades.made.push((d, text.clone()));
    super::note(w, text.clone(), Some(id));
    Ok(text)
}
