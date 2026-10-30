//! Your NFL desk (lane 10): contracts and talks with agents, tags and
//! tenders, fifth-year options, offer sheets, free agency with its visits,
//! restructures and releases. Every deal goes through the market's own
//! code: the player weighs your offer with the same `score` against the
//! same hidden ideal as any AI club's, signs through `sign_offer` or
//! `extend`, and in free agency your offer sits on the table beside the
//! AI clubs' in `market_day`, where the best one wins.
//!
//! What you see is what a real GM hears: the agent's ask, his claims (some
//! of them bluffs, which your negotiating reads), your cap manager's read
//! of the market, never the player's hidden ideal.

use crate::calendar::Date;
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::folk::voice::Facts;
use crate::world::pro::market::{self, Ideal, Offer, Sheet};
use crate::world::pro::{self, DeadWhy, Status, money};
use crate::world::{Stage, World};

use super::{Duty, Owner};

/// What kind of talk it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TalkKind {
    /// Your own player whose deal runs out at the new league year.
    ReSign,
    /// A player under contract: new years added on.
    Extension,
    /// A free agent on the open market (or in the negotiating window).
    FreeAgent,
    /// Another club's restricted free agent: an offer sheet.
    OfferSheet,
}

/// Where a talk stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TalkState {
    Open,
    /// Your offer is on the table in free agency, waiting on his decision.
    Standing,
    /// A verbal agreement in the negotiating window, signed when the league
    /// year opens.
    Agreed,
    Signed,
    /// His agent ended talks.
    Broken,
    /// He signed somewhere else, or the window closed.
    Gone,
}

/// A deal as you write it: the headline numbers and how it is laid out.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Proposal {
    pub years: u32,
    /// New money a year.
    pub apy: f64,
    /// Practical guarantee (signing bonus plus guaranteed base) as a share of the total.
    pub gtd: f64,
    /// Signing bonus as a share of the total (part of the guarantee).
    pub bonus: f64,
    /// Void years added after the real ones to spread the bonus (from 2016).
    pub void_years: u32,
    /// Not likely to be earned incentives a year.
    pub incentives: f64,
    pub no_trade: bool,
    /// A starting job promised.
    pub start: bool,
    /// How much each year's base grows over the last: 0 even, 0.06 rising,
    /// 0.15 back-loaded (real deals keep year one low with a big bonus and
    /// raise the base after).
    #[serde(default = "rising")]
    pub ramp: f64,
    /// Year one's base at the veteran minimum, the money in the later years.
    #[serde(default)]
    pub low_first: bool,
}

fn rising() -> f64 {
    0.06
}

impl Proposal {
    pub fn total(&self) -> f64 {
        self.apy * self.years as f64
    }

    fn offer(&self, team: usize) -> Offer {
        Offer { team, apy: self.apy, years: self.years, gtd: self.gtd, start: self.start }
    }

    /// The deal's layout, the way `terms_for` lays out the AI's (roster
    /// and workout bonuses, a back-loaded base), with your bonus, void
    /// years, incentives and no-trade clause.
    pub fn terms(&self, year: i32, age: u32) -> money::Terms {
        let mut t = money::Terms::flat(self.years, self.apy);
        t.bonus_share = self.bonus.min(self.gtd);
        t.base_guarantee = (self.gtd - t.bonus_share).max(0.0);
        t.injury_guarantee = if self.years >= 3 { (0.08f64).min(1.0 - self.gtd).max(0.0) } else { 0.0 };
        t.ramp = if self.years >= 2 { self.ramp.clamp(0.0, 0.3) } else { 0.0 };
        t.roster_share = if self.years >= 3 && year >= 1994 { 0.05 } else { 0.0 };
        t.workout_share = if year >= 1994 { 0.01 } else { 0.0 };
        t.per_game_share = if age >= 30 && year >= 2006 { 0.04 } else { 0.0 };
        t.incentives = self.incentives;
        t.void_years = if void_years_allowed(year) { self.void_years.min(4) } else { 0 };
        t.no_trade = self.no_trade;
        if self.low_first && self.years >= 2 {
            // The top minimum, so it clears any player's own.
            t.first_base = money::minimum(year, 7);
        }
        t.limit_first_year(money::price_cap(year) * money::FIRST_YEAR_MAX);
        t
    }
}

/// Void years became a cap tool once the 2011 CBA's proration rules were
/// read for them; clubs used them widely from about 2016 (proposed, not
/// confirmed: the same year the AI's GMs start).
pub fn void_years_allowed(year: i32) -> bool {
    year >= 2016
}

/// One line in a talk's history.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Line {
    pub date: Date,
    /// True for what you said, false for the agent.
    pub ours: bool,
    pub text: String,
}

/// A negotiation with one player's agent.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Talk {
    pub player: u32,
    pub kind: TalkKind,
    pub state: TalkState,
    pub opened: Date,
    /// Your offer on the table.
    pub offer: Option<Proposal>,
    /// The agent's last ask.
    pub ask: Option<Proposal>,
    /// His patience with the talks, 0-1: lowball offers wear it down.
    pub patience: f32,
    pub lines: Vec<Line>,
    /// Until when the agent won't take your call after ending talks.
    pub cold_until: Option<Date>,
}

/// Your NFL club's standing choices and talks.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct NflDesk {
    pub talks: Vec<Talk>,
    /// Fifth-year option choices by player id.
    pub options: Vec<(u32, bool)>,
    /// Offer sheet choices on your restricted players: match or not.
    pub sheets: Vec<(u32, bool)>,
    /// Offer sheets you were told about (player id, league year).
    pub sheets_told: Vec<(u32, i32)>,
    /// Free agents who visited your building (player id, date).
    pub visits: Vec<(u32, Date)>,
    /// Exclusive franchise tags you gave (player id, league year).
    pub exclusive: Vec<(u32, i32)>,
    /// June 1 designations used this league year (player id, league year).
    pub june1: Vec<(u32, i32)>,
    /// Reminders already sent (what, league year).
    pub reminded: Vec<(u8, i32)>,
    /// Tags and tenders placed before the new league year, with the deal
    /// they replaced, so one can be withdrawn until the league files it.
    pub placed: Vec<Placed>,
}

/// A tag or tender placed before the league year and what it replaced.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Placed {
    pub player: u32,
    pub year: i32,
    pub old: money::Contract,
    pub rfa: Option<money::Tender>,
    pub tags: u32,
}

impl NflDesk {
    pub fn talk(&self, id: u32) -> Option<&Talk> {
        self.talks.iter().find(|t| t.player == id && !matches!(t.state, TalkState::Signed | TalkState::Gone))
    }
    fn talk_mut(&mut self, id: u32) -> Option<&mut Talk> {
        self.talks.iter_mut().find(|t| t.player == id && !matches!(t.state, TalkState::Signed | TalkState::Gone))
    }
}

/// How the agent answers an offer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum Verdict {
    /// He signs today.
    Signed,
    /// A verbal agreement (the negotiating window): it signs at the league year.
    Agreed,
    /// In free agency: your offer is on the table against the others.
    Standing,
    /// Not there yet: the agent's ask comes back.
    Counter,
    /// The agent ends talks.
    Broken,
}

/// The agent's answer, as you hear it.
#[derive(Clone, Debug, serde::Serialize)]
pub struct Answer {
    pub verdict: Verdict,
    pub text: String,
    pub ask: Option<Proposal>,
    /// A claim about other interest, and your read of it in words.
    pub claim: Option<String>,
    pub read: Option<String>,
    /// How close you are, in his agent's tone: "far apart" to "there".
    pub gap: &'static str,
}

/// Your club.
fn club(w: &World) -> Result<usize, String> {
    super::club(w).ok_or_else(|| "You don't run an NFL club.".to_string())
}

fn person(w: &World, id: u32) -> Result<usize, String> {
    super::index_of(w, id).ok_or_else(|| "No such player.".to_string())
}

/// Your negotiating, 1-20 (your general manager's skill when you have none).
pub fn negotiating(w: &World) -> f64 {
    let Some(c) = w.career.as_ref() else { return 10.0 };
    w.coaches.get(c.coach).and_then(|co| co.yours).map_or(10.0, |y| y.negotiating)
}

/// What talk a player is open to with your club today, if any.
pub fn negotiable(w: &World, i: usize) -> Option<TalkKind> {
    let t = super::club(w)?;
    let year = w.year;
    let p = &w.people[i];
    if p.stage != Stage::Pro {
        return None;
    }
    let c = p.pro.as_ref()?;
    let league_year = w.dates.league_year.date;
    let opened = w.dates.tampering_open.map_or(league_year, |m| m.date);
    let k = c.contract.as_ref();
    if c.team == Some(t) && k.is_some_and(|k| k.team == t) {
        let k = k.unwrap();
        // Before the market: your own expiring player (a restricted one
        // gets a tender, not talks, until the tender is on him).
        if k.last_year() < year && w.today < league_year {
            if year >= 1993 && c.accrued < money::ufa_seasons(year) && c.rfa.is_none() {
                return None;
            }
            return Some(TalkKind::ReSign);
        }
        if matches!(k.kind, money::Kind::Minimum | money::Kind::Future | money::Kind::PracticeSquad) {
            return None;
        }
        let tagged = matches!(k.kind, money::Kind::Franchise | money::Kind::Transition);
        if tagged && w.today > w.dates.tag_long_term_deadline.date {
            return None;
        }
        if k.signed >= year && !tagged {
            return None;
        }
        let seasons = c.seasons.iter().filter(|s| s.games > 0).count();
        if k.kind == money::Kind::Rookie && year >= 2011 && seasons < 3 {
            return None;
        }
        return Some(TalkKind::Extension);
    }
    // A free agent off the street (released, unsigned) can sign any day,
    // in the weeks before the league year too (real: reserve/future deals).
    if c.status == Status::FreeAgent && c.team.is_none() && !c.on_waivers && !market::on_market(w, i, false) {
        return Some(TalkKind::FreeAgent);
    }
    // Free agents: from the negotiating window (another club's expiring
    // UFA) or the open market.
    let tampering = w.today >= opened && w.today < league_year;
    if c.team != Some(t) && (tampering || w.today >= league_year) {
        let on = if tampering {
            c.status == Status::Roster
                && c.accrued >= money::ufa_seasons(year)
                && k.is_some_and(|k| k.last_year() < year)
        } else {
            c.status == Status::FreeAgent && !c.on_waivers && c.team.is_none()
        };
        if on {
            return Some(TalkKind::FreeAgent);
        }
        // Another club's tendered restricted player: an offer sheet until
        // the RFA deadline.
        if !tampering
            && w.today <= w.dates.rfa_deadline
            && c.rfa.is_some()
            && c.team.is_some()
            && k.is_some_and(|k| k.kind == money::Kind::Tender)
        {
            return Some(TalkKind::OfferSheet);
        }
    }
    None
}

/// The player's ideal as the market code builds it (hidden from you).
fn ideal_of(w: &World, i: usize, kind: TalkKind) -> Ideal {
    if let Some(id) = w.people[i].pro.as_ref().and_then(|c| c.ideal)
        && kind == TalkKind::FreeAgent
    {
        return id;
    }
    let mut id = market::ideal(w, i, w.today);
    if kind == TalkKind::Extension
        && let Some(k) = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref())
    {
        // An early deal's security costs a little (as the AI's talks).
        id.apy *= 1.0 - 0.04 * (k.last_year() - w.year).max(0) as f64;
    }
    id
}

/// How an offer scores with him: the market's own score, with what a visit
/// to your building told both sides (a player who liked what he saw leans
/// your way a little, by your relationships; proposed, not confirmed: up
/// to 3%).
fn offer_score(w: &World, i: usize, o: &Offer, ideal: &Ideal, own: Option<usize>) -> f64 {
    let s = market::score(w, i, o, ideal, own);
    let id = w.people[i].player.id;
    let visited = w.front_office.nfl.visits.iter().any(|v| v.0 == id && v.1.days_to(w.today) <= 60);
    if !visited {
        return s;
    }
    let rel = w.career.as_ref().map_or(10.0, |c| w.coaches[c.coach].ability().relationships);
    s * (1.0 + 0.03 * clamp((rel - 4.0) / 16.0, 0.0, 1.0))
}

/// The bar a player's own team must clear before the market, the same as
/// the AI's talks: 85%, a little lower at the tag deadline, higher for a
/// money-minded player with no tie to the club who wants to see the market.
fn own_bar(w: &World, i: usize, kind: TalkKind) -> f64 {
    let g = market::goals(w, i);
    let deadline = w.today >= w.dates.tag_deadline.date.add(-2) && w.today <= w.dates.tag_deadline.date;
    match kind {
        TalkKind::Extension => {
            if deadline {
                0.82
            } else {
                0.87
            }
        }
        _ => (if deadline { 0.82 } else { 0.85 }) + 0.12 * g.money * (1.0 - g.loyalty),
    }
}

/// Rounds to a quarter million (agents talk in round numbers).
fn round_q(x: f64) -> f64 {
    if x >= 2e6 { (x / 250_000.0).round() * 250_000.0 } else { (x / 25_000.0).round() * 25_000.0 }
}

/// The agent's ask from the hidden ideal: a little over it (agents open
/// high, the greedy ones higher), his years and guarantees.
fn agent_ask(w: &World, i: usize, ideal: &Ideal) -> Proposal {
    let g = market::goals(w, i);
    let apy = round_q(ideal.apy * (1.02 + 0.06 * g.greed));
    let gtd = (ideal.gtd + 0.03).min(0.95);
    Proposal {
        years: ideal.years,
        apy,
        gtd: (gtd * 20.0).round() / 20.0,
        bonus: ((gtd * 0.55).min(0.35) * 20.0).round() / 20.0,
        void_years: 0,
        incentives: 0.0,
        no_trade: false,
        start: ideal.start,
        ramp: rising(),
        low_first: false,
    }
}

fn gap_words(s: f64) -> &'static str {
    if s >= 1.0 {
        "there"
    } else if s >= 0.85 {
        "close"
    } else if s >= 0.72 {
        "apart"
    } else {
        "far apart"
    }
}

/// Checks an offer against the rules before it goes out: the minimum for
/// his seasons, a bonus that fits the guarantee, the room it needs.
fn check(w: &World, t: usize, i: usize, p: &Proposal, kind: TalkKind) -> Result<(), String> {
    let year = w.year;
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    if p.years == 0 || p.years > 7 {
        return Err("A deal runs one to seven years.".into());
    }
    if p.apy < money::minimum(year, accrued) {
        return Err(format!("The minimum for his seasons is {}.", super::dollars(money::minimum(year, accrued))));
    }
    if !(0.0..=1.0).contains(&p.gtd) || p.bonus > p.gtd + 1e-9 {
        return Err("The signing bonus is part of the guarantee: it can't be larger.".into());
    }
    let limit = super::roster::roster_limit(w);
    if matches!(kind, TalkKind::FreeAgent | TalkKind::OfferSheet) && pro::count(w, t, Status::Roster) >= limit {
        return Err(format!("Your roster is full ({limit}): release someone first."));
    }
    if p.void_years > 0 && !void_years_allowed(year) {
        return Err("Void years aren't used in this era.".into());
    }
    // Year one's cap number must fit (an extension's new money starts next year).
    if money::capped(year) {
        let start = if kind == TalkKind::Extension { year + 1 } else { year };
        let age = w.people[i].player.age;
        let deal = money::build(t, money::Kind::Veteran, start, &p.terms(start, age));
        let hit = deal.cap_hit(start);
        // An extension frees the old pay for the years it covers; the old
        // bonus spread stays. Your own expiring player's void years land
        // on this year's cap when his new deal replaces the old one.
        let freed = if kind == TalkKind::Extension {
            w.people[i]
                .pro
                .as_ref()
                .and_then(|c| c.contract.as_ref())
                .map_or(0.0, |k| k.cap_hit(start) - k.year(start).map_or(0.0, |y| y.proration))
        } else if kind == TalkKind::ReSign {
            -pro::void_ahead(w, i)
        } else {
            0.0
        };
        let room = pro::cap_room(w, t, start) + freed - committed_elsewhere(w, t, i);
        if hit > room + 1.0 {
            return Err(format!(
                "Year one counts {} against the cap and you have {} of room.",
                super::dollars(hit),
                super::dollars(room.max(0.0))
            ));
        }
    }
    Ok(())
}

/// Money already promised in your other standing offers and agreements.
fn committed_elsewhere(w: &World, t: usize, i: usize) -> f64 {
    let id = w.people[i].player.id;
    let standing: f64 = w
        .front_office
        .nfl
        .talks
        .iter()
        .filter(|x| x.player != id && matches!(x.state, TalkState::Standing | TalkState::Agreed))
        .filter_map(|x| x.offer.map(|o| o.apy * 0.75))
        .sum();
    let pending: f64 =
        w.market.pending.iter().filter(|p| p.offer.team == t && p.person != i).map(|p| p.offer.apy * 0.75).sum();
    standing.max(pending)
}

/// Makes an offer to a player's agent and hears his answer.
pub fn propose(w: &mut World, id: u32, p: Proposal) -> Result<Answer, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let kind = negotiable(w, i).ok_or("He isn't someone you can talk terms with today.")?;
    // Acting on a duty takes it back from your staff.
    w.front_office.set_owner(kind_duty(kind), Owner::You);
    check(w, t, i, &p, kind)?;
    let today = w.today;
    if let Some(cold) = w.front_office.nfl.talk(id).and_then(|x| x.cold_until)
        && today < cold
    {
        return Err(format!("His agent won't take your call until {}.", cold.pretty()));
    }
    // Open the talk.
    if w.front_office.nfl.talk(id).is_none() {
        w.front_office.nfl.talks.push(Talk {
            player: id,
            kind,
            state: TalkState::Open,
            opened: today,
            offer: None,
            ask: None,
            patience: 1.0,
            lines: Vec::new(),
            cold_until: None,
        });
    }
    let ideal = ideal_of(w, i, kind);
    let own = match kind {
        TalkKind::ReSign | TalkKind::Extension => Some(t),
        _ => w.people[i].pro.as_ref().and_then(|c| c.teams.last().copied()),
    };
    let o = p.offer(t);
    let s = offer_score(w, i, &o, &ideal, own);
    let name = w.people[i].name.clone();
    let said = describe_proposal(&p, w.year);
    let ask = agent_ask(w, i, &ideal);
    let g = market::goals(w, i);
    let mut answer = Answer {
        verdict: Verdict::Counter,
        text: String::new(),
        ask: None,
        claim: None,
        read: None,
        gap: gap_words(s),
    };
    match kind {
        TalkKind::ReSign | TalkKind::Extension => {
            if s >= own_bar(w, i, kind) || w.year < 1993 && kind == TalkKind::ReSign {
                answer.verdict = Verdict::Signed;
                answer.text = format!("You have a deal. {name} signs: {said}.");
            }
        }
        TalkKind::FreeAgent
            if free_now(w, i) && (!market::on_market(w, i, false) || today >= w.dates.veterans_report) =>
        {
            // The street, and anyone unsigned once camps open: no market
            // day weighs offers, so he answers yours at once. (A player in
            // the negotiating window is still under his old deal: his
            // answer waits for the market's day.)
            let street = !market::on_market(w, i, false);
            let yes = if street { p.apy >= pro::ask(w, i, w.year, today).0 * 0.95 } else { s >= 0.85 };
            if yes && (street || today >= w.dates.league_year.date) {
                answer.verdict = Verdict::Signed;
                answer.text = format!("{name} takes it: {said}.");
            }
        }
        TalkKind::FreeAgent | TalkKind::OfferSheet => {
            answer.verdict = Verdict::Standing;
        }
    }
    if answer.verdict == Verdict::Counter || answer.verdict == Verdict::Standing {
        // Lowball offers wear his patience down, the hot-tempered fastest.
        let hurt = if s < 0.55 {
            0.45
        } else if s < 0.7 {
            0.25
        } else {
            0.0
        } * (0.6 + 0.8 * g.temper);
        let talk = w.front_office.nfl.talk_mut(id).unwrap();
        talk.patience = (talk.patience - hurt as f32).max(0.0);
        if talk.patience <= 0.0 {
            answer.verdict = Verdict::Broken;
        }
    }
    // What the agent says.
    if answer.verdict != Verdict::Signed && answer.verdict != Verdict::Broken {
        answer.ask = Some(ask);
        let (claim, read) = agent_claim(w, i, t, kind, s, &ideal);
        answer.claim = claim;
        answer.read = read;
        // In the agent's own voice (lane 14, D10); the numbers are the market's.
        let f = Facts::new().with("name", name.clone()).with("money", super::dollars(ask.apy));
        let (part, plain) = match kind {
            TalkKind::FreeAgent | TalkKind::OfferSheet if s >= 1.0 => (
                "yes",
                format!(
                    "\"That works for {name}. He'll give everyone until the next market day, but you're where he wants to be.\""
                ),
            ),
            TalkKind::FreeAgent | TalkKind::OfferSheet if s >= 0.85 => {
                ("range", format!("\"That's in range. {name} will weigh it against what else comes in.\""))
            }
            TalkKind::FreeAgent | TalkKind::OfferSheet if s >= 0.72 => {
                ("apart", format!("\"We're still apart on this. Here's where {name} is.\""))
            }
            TalkKind::FreeAgent | TalkKind::OfferSheet => {
                ("far", format!("\"We're a long way apart on this. Here's where {name} is.\""))
            }
            _ if s >= 0.85 => {
                ("near", format!("\"We're close. Get us to {} and {name} signs.\"", super::dollars(ask.apy)))
            }
            _ if s >= 0.72 => ("short", format!("\"That's short of what {name} is worth. Here's our number.\"")),
            _ => ("insult", format!("\"Respectfully, that isn't a serious offer for {name}.\"")),
        };
        answer.text = crate::world::folk::dialogue::agent_says(w, i, part, &f, plain);
    }
    if answer.verdict == Verdict::Broken {
        let cold = today.add(if kind == TalkKind::FreeAgent { 7 } else { 30 });
        let f = Facts::new().with("name", name.clone());
        let said = crate::world::folk::dialogue::agent_says(
            w,
            i,
            "done",
            &f,
            format!("\"We're done for now. {name} will look elsewhere.\""),
        );
        answer.text = format!("{said} His agent ended the talks.");
        let talk = w.front_office.nfl.talk_mut(id).unwrap();
        talk.state = TalkState::Broken;
        talk.cold_until = Some(cold);
        if kind == TalkKind::ReSign {
            w.people[i].note(format!("Broke off talks with the {} before free agency.", w.nfl[t].name));
        }
    }
    // Write the talk's history.
    {
        let talk = w.front_office.nfl.talks.iter_mut().rev().find(|x| x.player == id).unwrap();
        talk.lines.push(Line { date: today, ours: true, text: said.clone() });
        talk.lines.push(Line { date: today, ours: false, text: answer.text.clone() });
        // The same claim again isn't news: it's written once.
        if let Some(c) = &answer.claim
            && !talk.lines.iter().any(|l| !l.ours && &l.text == c)
        {
            talk.lines.push(Line { date: today, ours: false, text: c.clone() });
        }
        talk.ask = answer.ask;
        talk.offer = Some(p);
        if talk.state != TalkState::Broken {
            talk.state = match answer.verdict {
                Verdict::Standing => TalkState::Standing,
                Verdict::Agreed => TalkState::Agreed,
                Verdict::Signed => TalkState::Signed,
                _ => TalkState::Open,
            };
        }
    }
    if answer.verdict == Verdict::Signed {
        sign_now(w, i, t, kind, &p);
    } else if kind == TalkKind::OfferSheet && s >= 0.85 {
        offer_sheet(w, i, t, &p);
        answer.text = format!("{name} signs your offer sheet. His club has five days to match.");
    }
    Ok(answer)
}

/// A free agent no club holds: off the street or on the open market.
fn free_now(w: &World, i: usize) -> bool {
    w.people[i].pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent && c.team.is_none() && !c.on_waivers)
}

fn kind_duty(k: TalkKind) -> Duty {
    match k {
        TalkKind::ReSign | TalkKind::Extension => Duty::Contracts,
        TalkKind::FreeAgent | TalkKind::OfferSheet => Duty::FreeAgency,
    }
}

/// What the agent claims about other interest, and your read of it. An
/// honest agent tells you what's out there; a less honest one claims
/// interest that isn't there (the market's bluff, topic 3). Your
/// negotiating reads it the way the AI GM's skill does in `bluff`.
fn agent_claim(
    w: &World,
    i: usize,
    t: usize,
    kind: TalkKind,
    s: f64,
    ideal: &Ideal,
) -> (Option<String>, Option<String>) {
    let Some(a) = w.people[i].pro.as_ref().and_then(|c| c.agent) else { return (None, None) };
    let agent = &w.market.agents[a];
    let id = w.people[i].player.id as u64;
    let day = w.today.0 as u64;
    if s >= 1.0 {
        return (None, None);
    }
    // Real interest: AI clubs whose own offer would beat yours.
    let rivals: Vec<(usize, f64)> = if kind == TalkKind::FreeAgent {
        let year = w.year;
        (0..w.clubs.len())
            .filter(|&o| o != t && crate::world::franchise::active_in(w, o, year))
            .filter_map(|o| market::make_offer(w, o, i, ideal, 20, 0.0, year).map(|x| (o, x.apy)))
            .collect()
    } else {
        Vec::new()
    };
    let best = rivals.iter().max_by(|a, b| a.1.total_cmp(&b.1)).copied();
    let bluffs = super::key(w, 0xB1A0 ^ day, id) > 1.0 - (1.0 - agent.honesty) * 0.8;
    let name = w.people[i].name.clone();
    let (part, f, plain, true_claim) = match (kind, best, bluffs) {
        (TalkKind::FreeAgent, Some((o, apy)), _) => {
            let money = super::dollars(round_q(apy));
            let club = w.nfl[o].name;
            let plain = format!("\"The {club} are at about {money} a year.\"");
            ("claim.real", Facts::new().with("club", club).with("money", money), plain, true)
        }
        (TalkKind::FreeAgent, None, true) => {
            let money = super::dollars(round_q(ideal.apy * 1.05));
            let plain = format!("\"Two other clubs are above you, near {money} a year.\"");
            ("claim.bluff", Facts::new().with("money", money), plain, false)
        }
        (TalkKind::ReSign, _, true) => {
            let money = super::dollars(round_q(ideal.apy * 1.12));
            let plain = format!("\"On the open market {name} gets {money} a year, easily.\"");
            ("claim.market", Facts::new().with("name", name.clone()).with("money", money), plain, false)
        }
        (TalkKind::ReSign, _, false) if ideal.apy > money::minimum(w.year, 4) * 3.0 => {
            let plain =
                "\"He knows what the market pays, and he'd like to stay. Meet us near our number.\"".to_string();
            ("claim.stay", Facts::new().with("name", name.clone()), plain, true)
        }
        _ => return (None, None),
    };
    let text = crate::world::folk::dialogue::agent_says(w, i, part, &f, plain);
    // Your read: right more often the better you negotiate and the more
    // you know this agent (his history with your club).
    let rel = agent.rel.get(t).copied().unwrap_or(0.0);
    let skill = negotiating(w);
    let read_p = clamp(0.45 + 0.03 * (skill - 10.0) + 0.15 * rel.abs(), 0.2, 0.95);
    let right = super::key(w, 0xB1A1 ^ day, id) < read_p;
    let says_real = if right { true_claim } else { !true_claim };
    let sure = if read_p > 0.75 {
        "fairly sure"
    } else if read_p > 0.55 {
        "not certain"
    } else {
        "a guess"
    };
    let read = if says_real {
        format!("Your read: it sounds real ({sure}).")
    } else {
        format!("Your read: he's bluffing ({sure}).")
    };
    (Some(text), Some(read))
}

/// Your words for a proposal: "4 years, $48.0M ($12.0M a year), $26.4M guaranteed".
pub fn describe_proposal(p: &Proposal, _year: i32) -> String {
    let mut s = format!(
        "{} year{}, {} ({} a year), {} guaranteed",
        p.years,
        if p.years == 1 { "" } else { "s" },
        super::dollars(p.total()),
        super::dollars(p.apy),
        super::dollars(p.total() * p.gtd)
    );
    if p.bonus > 0.0 {
        s += &format!(", {} signing bonus", super::dollars(p.total() * p.bonus));
    }
    if p.void_years > 0 {
        s += &format!(", {} void year{}", p.void_years, if p.void_years == 1 { "" } else { "s" });
    }
    if p.incentives > 0.0 {
        s += &format!(", up to {} a year in incentives", super::dollars(p.incentives));
    }
    if p.no_trade {
        s += ", a no-trade clause";
    }
    if p.start {
        s += ", a starting job promised";
    }
    s
}

/// Signs a deal agreed with your own player through the market's own code.
fn sign_now(w: &mut World, i: usize, t: usize, kind: TalkKind, p: &Proposal) {
    let d = w.today;
    let o = p.offer(t);
    let year = w.year;
    let id = w.people[i].player.id;
    match kind {
        TalkKind::Extension => {
            market::extend(w, i, &o, d);
            w.market.year_mut(year).extensions += 1;
        }
        TalkKind::FreeAgent => {
            market::sign_offer(w, i, &o, d, money::Kind::Veteran, "talks with your front office");
            w.market.year_mut(year).signed_new += 1;
        }
        _ => {
            market::sign_offer(w, i, &o, d, money::Kind::Veteran, "talks with your front office");
            w.market.year_mut(year).resigned += 1;
        }
    }
    if let Some(c) = w.people[i].pro.as_mut() {
        c.hold = market::Hold::None;
        c.rfa = None;
    }
    let name = w.people[i].name.clone();
    super::note(w, format!("Signed {name}: {}.", describe_proposal(p, year)), Some(id));
}

/// Your offer sheet to another club's restricted free agent: the same
/// sheet the AI's clubs sign, five days for his club to match.
fn offer_sheet(w: &mut World, i: usize, t: usize, p: &Proposal) {
    let Some(c) = w.people[i].pro.as_ref() else { return };
    let (Some(old), Some(tender)) = (c.team, c.rfa) else { return };
    let until = w.today.add(5);
    w.market.sheets.push(Sheet { person: i, offer: p.offer(t), old, tender, until });
    let name = w.people[i].name.clone();
    let id = w.people[i].player.id;
    super::note(w, format!("Offer sheet to {name} ({}).", describe_proposal(p, w.year)), Some(id));
}

/// Your terms for a deal the market code is about to sign for your club.
pub(crate) fn seat_terms(w: &World, i: usize, o: &Offer, start: i32) -> Option<money::Terms> {
    let t = super::club(w)?;
    if o.team != t {
        return None;
    }
    let id = w.people[i].player.id;
    let talk = w.front_office.nfl.talks.iter().rev().find(|x| x.player == id)?;
    let p = talk.offer?;
    // Only the deal you offered (a matched sheet, an AI-made deal for a
    // duty you handed over keep the staff's own layout).
    if (p.apy - o.apy).abs() > 1.0 || p.years != o.years {
        return None;
    }
    Some(p.terms(start, w.people[i].player.age))
}

/// Your standing offer to a free agent, for the market's day: the offer and
/// how it scores with him, beside the AI clubs'.
pub(crate) fn seat_offer(
    w: &World,
    i: usize,
    ideal: &Ideal,
    former: Option<usize>,
    committed: &[f64],
) -> Option<(Offer, f64)> {
    let t = super::club(w)?;
    if w.front_office.nfl.talks.is_empty() {
        return None;
    }
    let id = w.people[i].player.id;
    let talk = w.front_office.nfl.talks.iter().find(|x| {
        x.player == id && x.kind == TalkKind::FreeAgent && matches!(x.state, TalkState::Standing | TalkState::Agreed)
    })?;
    let p = talk.offer?;
    let year = w.year;
    if pro::count(w, t, Status::Roster) >= super::roster::roster_limit(w) {
        return None;
    }
    if money::capped(year) && pro::cap_room(w, t, year) - committed.get(t).copied().unwrap_or(0.0) < p.apy * 0.75 {
        return None;
    }
    let o = p.offer(t);
    Some((o, offer_score(w, i, &o, ideal, former)))
}

/// Your answer to an offer sheet on one of your restricted players.
pub(crate) fn sheet_match(w: &World, s: &Sheet) -> Option<bool> {
    let t = super::club(w)?;
    if s.old != t {
        return None;
    }
    let id = w.people[s.person].player.id;
    // Undecided at the deadline: not matched, as the rule has it.
    Some(w.front_office.nfl.sheets.iter().find(|x| x.0 == id).is_some_and(|x| x.1))
}

/// Your fifth-year option choice on a player of yours.
pub(crate) fn option_choice(w: &World, t: usize, i: usize) -> Option<bool> {
    if !super::yours(w, t, Duty::Contracts) {
        return None;
    }
    let id = w.people[i].player.id;
    Some(w.front_office.nfl.options.iter().find(|x| x.0 == id).is_some_and(|x| x.1))
}

/// Sets a fifth-year option choice (it takes effect on the deadline).
pub fn set_option(w: &mut World, id: u32, take: bool) -> Result<(), String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let ok = w.people[i]
        .pro
        .as_ref()
        .filter(|c| c.team == Some(t))
        .and_then(|c| c.contract.as_ref().map(|k| (k, c.draft)))
        .is_some_and(|(k, dr)| k.kind == money::Kind::Rookie && k.option.is_none() && dr.is_some_and(|x| x.1 == 1));
    if !ok || w.year < 2011 {
        return Err("He has no fifth-year option to decide.".into());
    }
    if w.today > w.dates.fifth_year_option {
        return Err("The option deadline has passed.".into());
    }
    w.front_office.set_owner(Duty::Contracts, Owner::You);
    w.front_office.nfl.options.retain(|x| x.0 != id);
    w.front_office.nfl.options.push((id, take));
    let name = w.people[i].name.clone();
    super::note(
        w,
        format!("{} {name}'s fifth-year option.", if take { "Will pick up" } else { "Will decline" }),
        Some(id),
    );
    Ok(())
}

/// Sets your answer to an offer sheet on your player.
pub fn set_sheet(w: &mut World, id: u32, matched: bool) -> Result<(), String> {
    let t = club(w)?;
    let i = person(w, id)?;
    if !w.market.sheets.iter().any(|s| s.person == i && s.old == t) {
        return Err("There's no offer sheet on him.".into());
    }
    w.front_office.nfl.sheets.retain(|x| x.0 != id);
    w.front_office.nfl.sheets.push((id, matched));
    Ok(())
}

// --- Tags and tenders ---------------------------------------------------------------------------

/// The tag prices for one of your expiring players this year: (non-
/// exclusive franchise, exclusive franchise, transition). The exclusive
/// tag reads this year's top five cap numbers at the position (real:
/// the current year's top five, never the five-year formula).
pub fn tag_prices(w: &World, i: usize) -> (f64, f64, f64) {
    let year = w.year;
    let pos = w.people[i].player.position;
    let mut hist = w.market.tag_shares.clone();
    if !hist.iter().any(|(y, _)| *y == year) {
        hist.push((year, market::tag_shares_now(w)));
        hist.retain(|(y, _)| *y > year - 5);
    }
    let bases = market::tag_bases(&hist, year);
    let qb = bases[Position::QB.index()][0];
    let c = w.people[i].pro.as_ref();
    let own = c.and_then(|c| c.contract.as_ref()).map_or(0.0, |k| k.cap_hit(year - 1));
    let times = c.map_or(0, |c| c.tags);
    let franchise = money::tag_repeat(bases[pos.index()][0], own, times, if times >= 2 { qb } else { 0.0 });
    let transition = money::tag_repeat(bases[pos.index()][1], own, times, 0.0);
    let top = market::top_cap_hits(w);
    let five = &top[pos.index()];
    let now = if five.is_empty() { 0.0 } else { five.iter().take(5).sum::<f64>() / five.len().min(5) as f64 };
    let exclusive = money::tag_repeat(now.max(bases[pos.index()][0]), own, times, if times >= 2 { qb } else { 0.0 });
    (franchise, exclusive, transition)
}

/// A round-one pick's fifth-year option price, as the market prices it.
pub fn option_price(w: &World, i: usize) -> f64 {
    market::option_price(w, i)
}

/// Whether the tag window is open today.
pub fn tag_window(w: &World) -> bool {
    w.year >= 1993 && w.today >= w.dates.tag_window_open && w.today <= w.dates.tag_deadline.date
}

/// Tags one of your expiring players: "franchise", "exclusive" or "transition".
pub fn tag(w: &mut World, id: u32, what: &str) -> Result<String, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    if !tag_window(w) {
        return Err("Tags can only be used in the tag window (two weeks before the deadline).".into());
    }
    let year = w.year;
    let c = w.people[i].pro.as_ref().ok_or("No such pro.")?;
    let k = c.contract.as_ref().ok_or("He has no contract.")?;
    if c.team != Some(t) || k.last_year() >= year {
        return Err("Only a player whose deal runs out can be tagged.".into());
    }
    if c.accrued < money::ufa_seasons(year) {
        return Err("He'd be a restricted free agent: tender him instead.".into());
    }
    let used: Vec<money::Kind> = w.clubs[t]
        .roster
        .iter()
        .filter_map(|&j| w.people[j].pro.as_ref()?.contract.as_ref())
        .filter(|k| k.signed == year && matches!(k.kind, money::Kind::Franchise | money::Kind::Transition))
        .map(|k| k.kind)
        .collect();
    let kind = if what == "transition" { money::Kind::Transition } else { money::Kind::Franchise };
    // One tag a club a year from 2011; a franchise and a transition tag before.
    if year >= 2011 && !used.is_empty() || year < 2011 && used.contains(&kind) {
        return Err(if year >= 2011 {
            "You've used this year's tag.".into()
        } else {
            "You've used this year's tag of that kind.".into()
        });
    }
    let (f, x, tr) = tag_prices(w, i);
    let price = match what {
        "transition" => tr,
        "exclusive" => x,
        _ => f,
    };
    let voids = pro::void_ahead(w, i);
    if money::capped(year) && pro::cap_room(w, t, year) < price + voids {
        return Err(if voids > 0.0 {
            format!(
                "The tag counts {} the moment it's placed, and his old deal's void years land {} more on this year's cap with it: you are {} short. Clear room on the cap sheet first.",
                super::dollars(price),
                super::dollars(voids),
                super::dollars(price + voids - pro::cap_room(w, t, year))
            )
        } else {
            format!(
                "The tag counts {} the moment it's placed: you are {} short. Clear room on the cap sheet first.",
                super::dollars(price),
                super::dollars(price - pro::cap_room(w, t, year))
            )
        });
    }
    let deal = money::build(t, kind, year, &{
        let mut x = money::Terms::flat(1, price);
        x.base_guarantee = 1.0;
        x
    });
    let name = w.nfl[t].name;
    let pname = w.people[i].name.clone();
    place(w, i);
    pro::settle_ended(w, i);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.contract = Some(deal);
    c.tags += 1;
    let age = p.row_age();
    let label = match what {
        "transition" => "transition",
        "exclusive" => "exclusive franchise",
        _ => "franchise",
    };
    p.note(format!("Given the {label} tag by the {name} at {age}: ${:.2}M for one year.", price / 1e6));
    if what == "exclusive" {
        w.front_office.nfl.exclusive.push((id, year));
    }
    let my = w.market.year_mut(year);
    if kind == money::Kind::Franchise {
        my.tags += 1
    } else {
        my.transition += 1
    }
    let who = pro::describe(w, i);
    market::post(
        w,
        w.today,
        Some(t),
        Some(i),
        crate::calendar::Loudness::Medium,
        crate::calendar::Kind::Signing,
        format!("{who} gets the {label} tag (${:.1}M).", price / 1e6),
    );
    w.front_office.set_owner(Duty::Tags, Owner::You);
    let text = format!("Tagged {pname} ({label}, {} for one year).", super::dollars(price));
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Tenders one of your restricted (or exclusive-rights) players: "first",
/// "second", "original", "rofr" or "erfa".
pub fn tender(w: &mut World, id: u32, level: &str) -> Result<String, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let year = w.year;
    if w.today > w.dates.league_year.date || w.year < 1993 {
        return Err("Tenders go out before the new league year opens.".into());
    }
    let c = w.people[i].pro.as_ref().ok_or("No such pro.")?;
    let k = c.contract.as_ref().ok_or("He has no contract.")?;
    if c.team != Some(t) || k.last_year() >= year {
        return Err("Only a player whose deal runs out can be tendered.".into());
    }
    // The season just played is counted at the Super Bowl.
    let accrued = c.accrued;
    if accrued >= money::ufa_seasons(year) {
        return Err("He'll be unrestricted: re-sign or tag him.".into());
    }
    let own = k.cap_hit(year - 1);
    let (deal_price, rfa, text) = if accrued < 3 || level == "erfa" {
        if accrued >= 3 {
            return Err("With three seasons he's restricted, not exclusive-rights.".into());
        }
        let p = money::minimum(year, c.accrued);
        (p, None, "an exclusive-rights tender")
    } else {
        let tend = match level {
            "first" => money::Tender::First,
            "second" => money::Tender::Second,
            "original" if c.draft.is_some() => money::Tender::Original,
            "original" | "rofr" => money::Tender::RightOfFirstRefusal,
            _ => return Err("Pick a tender level.".into()),
        };
        (
            money::tender_price(year, tend).max(own * 1.1),
            Some(tend),
            match tend {
                money::Tender::First => "a first-round tender",
                money::Tender::Second => "a second-round tender",
                money::Tender::Original => "an original-round tender",
                _ => "a right-of-first-refusal tender",
            },
        )
    };
    let voids = pro::void_ahead(w, i);
    if money::capped(year) && pro::cap_room(w, t, year) < deal_price + voids {
        return Err(format!(
            "The tender counts {} the moment it's placed{}: you are {} short. Clear room on the cap sheet first.",
            super::dollars(deal_price),
            if voids > 0.0 {
                format!(" (and his old deal's void years {} more)", super::dollars(voids))
            } else {
                String::new()
            },
            super::dollars(deal_price + voids - pro::cap_room(w, t, year))
        ));
    }
    let deal = money::build(t, money::Kind::Tender, year, &money::Terms::flat(1, deal_price));
    let name = w.nfl[t].name;
    let pname = w.people[i].name.clone();
    place(w, i);
    pro::settle_ended(w, i);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.contract = Some(deal);
    c.rfa = rfa;
    let age = p.row_age();
    p.note(format!("Given {text} by the {name} at {age} (${:.2}M).", deal_price / 1e6));
    if rfa.is_some() {
        w.market.year_mut(year).tenders += 1;
    }
    w.front_office.set_owner(Duty::Tags, Owner::You);
    let line = format!("Tendered {pname}: {text}, {}.", super::dollars(deal_price));
    super::note(w, line.clone(), Some(id));
    Ok(line)
}

/// Takes back a tag or tender before he signs it (the Panthers and Josh
/// Norman, 2016): he becomes a free agent and nothing is owed.
pub fn rescind(w: &mut World, id: u32) -> Result<String, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let year = w.year;
    let c = w.people[i].pro.as_ref().ok_or("No such pro.")?;
    let ok = c.team == Some(t)
        && c.contract.as_ref().is_some_and(|k| {
            k.signed == year && matches!(k.kind, money::Kind::Franchise | money::Kind::Transition | money::Kind::Tender)
        });
    if !ok {
        return Err("He has no tag or tender to take back.".into());
    }
    if w.today < w.dates.league_year.date {
        return withdraw(w, i, id);
    }
    let name = w.nfl[t].name;
    w.clubs[t].roster.retain(|&x| x != i);
    let p = &mut w.people[i];
    let c = p.pro.as_mut().unwrap();
    c.contract = None;
    c.team = None;
    c.status = Status::FreeAgent;
    c.former = Some(t);
    c.expired = true;
    c.rfa = None;
    c.ideal = None;
    let age = p.row_age();
    p.note(format!("The {name} took back his tag or tender at {age}: a free agent."));
    let pname = w.people[i].name.clone();
    let text = format!("Rescinded the tender on {pname}: he's a free agent.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Keeps the deal a tag or tender replaces, while the league year hasn't
/// opened.
fn place(w: &mut World, i: usize) {
    let year = w.year;
    let Some(c) = w.people[i].pro.as_ref() else { return };
    let Some(old) = c.contract.clone() else { return };
    if w.today >= w.dates.league_year.date || old.signed == year {
        return;
    }
    let id = w.people[i].player.id;
    let (rfa, tags) = (c.rfa, c.tags);
    let placed = &mut w.front_office.nfl.placed;
    placed.retain(|x| x.year >= year && x.player != id);
    placed.push(Placed { player: id, year, old, rfa, tags });
}

/// Before the league year a tag or tender is only your plan: withdrawing
/// it puts his old deal back, to run out at the league year as before.
fn withdraw(w: &mut World, i: usize, id: u32) -> Result<String, String> {
    let year = w.year;
    let k = w.front_office.nfl.placed.iter().position(|x| x.player == id && x.year == year);
    let Some(k) = k else { return Err("That tag or tender can be taken back once the new league year opens.".into()) };
    let x = w.front_office.nfl.placed.remove(k);
    let kind = w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).map(|k| k.kind);
    let tendered = w.people[i].pro.as_ref().is_some_and(|c| c.rfa.is_some());
    let team = x.old.team;
    let c = w.people[i].pro.as_mut().unwrap();
    c.contract = Some(x.old);
    c.rfa = x.rfa;
    c.tags = x.tags;
    // The void years' bonus goes back on his old deal.
    if let Some(club) = w.clubs.get_mut(team) {
        club.dead.retain(|d| !(d.player == id && d.year == year && d.why == pro::DeadWhy::Voided));
    }
    w.front_office.nfl.exclusive.retain(|e| !(e.0 == id && e.1 == year));
    let my = w.market.year_mut(year);
    let what = match kind {
        Some(money::Kind::Franchise) => {
            my.tags = my.tags.saturating_sub(1);
            "tag"
        }
        Some(money::Kind::Transition) => {
            my.transition = my.transition.saturating_sub(1);
            "tag"
        }
        _ => {
            if tendered {
                my.tenders = my.tenders.saturating_sub(1);
            }
            "tender"
        }
    };
    let pname = w.people[i].name.clone();
    let text = format!("Withdrew the {what} on {pname}: his deal runs out at the new league year unless you act.");
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

// --- The cap: restructures and releases ---------------------------------------------------------

/// A restructure: this year's base above the minimum becomes a bonus,
/// spread over the years left (void years added when you ask and the era
/// allows). Returns the room it frees this year.
pub fn restructure(w: &mut World, id: u32, voids: u32) -> Result<String, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let year = w.year;
    if !money::capped(year) {
        return Err("There's no cap this year to restructure for.".into());
    }
    let accrued = w.people[i].pro.as_ref().map_or(0, |c| c.accrued);
    let keep = money::minimum(year, accrued);
    let voids = if void_years_allowed(year) { voids.min(4) } else { 0 };
    let c = w.people[i].pro.as_mut().ok_or("No such pro.")?;
    if c.team != Some(t) {
        return Err("He isn't yours.".into());
    }
    let k = c.contract.as_mut().ok_or("He has no contract.")?;
    if k.last_year() <= year && voids == 0 {
        return Err("A deal in its last year has no years to spread a bonus over (add void years).".into());
    }
    // Later hits can't run past the real high (`money::CAP_HIT_MAX`).
    let mut trial = k.clone();
    let saved = money::restructure(&mut trial, year, keep, voids);
    if saved < 100_000.0 {
        return Err("There's little base salary above the minimum to convert.".into());
    }
    if trial.years.iter().any(|y| y.year > year && y.cap_hit() > money::price_cap(y.year) * money::CAP_HIT_MAX) {
        return Err("That would push a later year's cap number past anything a club has carried.".into());
    }
    *k = trial;
    let name = w.people[i].name.clone();
    let text = format!("Restructured {name}: {} of room this year, more on later years' caps.", super::dollars(saved));
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Releases a player now. `june1` designates the release post-June 1 (two
/// a year, real since 2011): his later proration lands next year.
pub fn release(w: &mut World, id: u32, june1: bool) -> Result<String, String> {
    let t = club(w)?;
    let i = person(w, id)?;
    let year = w.year;
    let today = w.today;
    let c = w.people[i].pro.as_ref().ok_or("No such pro.")?;
    if c.team != Some(t) {
        return Err("He isn't yours.".into());
    }
    if june1 {
        if pro::after_june1(w, today) {
            return Err("After June 1 every release splits its dead money already.".into());
        }
        let used = w.front_office.nfl.june1.iter().filter(|x| x.1 == year).count();
        if year >= 2011 && used >= 2 {
            return Err("You've used both post-June 1 designations this year.".into());
        }
    }
    let (now, later) =
        c.contract.as_ref().map_or((0.0, 0.0), |k| k.dead_if_cut(year, today, june1 || pro::after_june1(w, today)));
    let name = w.people[i].name.clone();
    pro::release_with(w, i, today, "released by your front office", june1, true, DeadWhy::Released);
    if june1 {
        w.front_office.nfl.june1.push((id, year));
    }
    let text = if later > 0.0 {
        format!("Released {name}: {} dead money this year, {} next year.", super::dollars(now), super::dollars(later))
    } else {
        format!("Released {name}: {} dead money.", super::dollars(now))
    };
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// Brings a free agent in for a visit (not in the negotiating window: the
/// rules allow talks, not visits). Your staff gets a medical and both
/// sides learn what matters: he tells you what he wants most.
pub fn visit(w: &mut World, id: u32) -> Result<String, String> {
    let _t = club(w)?;
    let i = person(w, id)?;
    if negotiable(w, i) != Some(TalkKind::FreeAgent) {
        return Err("He isn't a free agent you can bring in.".into());
    }
    if w.today < w.dates.league_year.date {
        return Err("Visits wait for the new league year; the negotiating window allows talks only.".into());
    }
    if w.front_office.nfl.visits.iter().any(|v| v.0 == id && v.1.days_to(w.today) <= 60) {
        return Err("He visited already.".into());
    }
    w.front_office.nfl.visits.push((id, w.today));
    let name = w.people[i].name.clone();
    let text = format!("{name} visited your building. {}", wants_words(w, i));
    super::note(w, text.clone(), Some(id));
    Ok(text)
}

/// What a player told you he wants most, in his words (from his hidden
/// goals; only after a visit or a long talk).
pub fn wants_words(w: &World, i: usize) -> String {
    let g = market::goals(w, i);
    let mut parts: Vec<(f64, &str)> = vec![
        (g.money * 1.6, "the money"),
        (g.security, "guarantees"),
        (g.winning * 0.7, "a contender"),
        (g.role * 0.7, "a starting job"),
        (g.city * 0.4, "the right city"),
        (g.home * 0.4, "being close to home"),
        (g.loyalty * 0.6, "loyalty to his last club"),
    ];
    parts.sort_by(|a, b| b.0.total_cmp(&a.0));
    format!("He says what matters most is {} and then {}.", parts[0].1, parts[1].1)
}

/// Your cap manager's read of a player's market: a range a year around
/// the league's price for him, narrower the better you negotiate.
pub fn market_read(w: &World, i: usize) -> (f64, f64) {
    let p = &w.people[i];
    let view = market::market_grade(w, p.player.position, market::consensus(w, i));
    let accrued = p.pro.as_ref().map_or(0, |c| c.accrued);
    let price = market::market_price(w.year, p.player.position, view, accrued, p.player.age)
        .max(money::minimum(w.year, accrued));
    let width = clamp(0.22 - 0.01 * (negotiating(w) - 10.0), 0.08, 0.35);
    let off = super::key_n(w, 0xCAB0, p.player.id as u64) * width * 0.3;
    let mid = price * (1.0 + off);
    (round_q(mid * (1.0 - width / 2.0)), round_q(mid * (1.0 + width / 2.0)))
}

// --- The days -----------------------------------------------------------------------------------

/// A reminder once per league year.
/// Takes a decision reminder off the pinned list once its deadline has
/// passed: the line stays in the inbox, it just no longer waits on you.
fn unpin(w: &mut World, from: Date, text: &str) {
    let today = w.today;
    for (_, evs) in w.calendar.events.range_mut(from..=today) {
        for e in evs.iter_mut() {
            if e.needs_decision && e.rule == "Your front office" && e.text.contains(text) {
                e.needs_decision = false;
            }
        }
    }
}

fn remind(w: &mut World, what: u8, text: String, decision: bool) {
    let year = w.year;
    if w.front_office.nfl.reminded.contains(&(what, year)) {
        return;
    }
    w.front_office.nfl.reminded.push((what, year));
    w.front_office.nfl.reminded.retain(|x| x.1 >= year - 1);
    // Read the next morning: an inbox line dated that morning; a decision
    // stays on today, so Continue stops before the morning comes.
    let d = if decision { w.today } else { w.today.add(1) };
    super::tell(w, d, text, decision);
}

/// Your desk's day: reminders before the deadlines, offer sheets on your
/// players, talks settled by the market's day.
pub(crate) fn day(w: &mut World, d: Date) {
    let Some(t) = super::club(w) else { return };
    let year = w.year;
    let y = w.dates.clone();
    // A reminder stays pinned while its deadline stands, not after.
    if d == y.tag_deadline.date.add(1) {
        unpin(w, y.tag_deadline.date.add(-5), "The tag deadline is");
    }
    if d == y.fifth_year_option.add(1) {
        unpin(w, y.fifth_year_option.add(-10), "Fifth-year options are due");
    }
    let sheets: Vec<usize> = w.market.sheets.iter().map(|s| s.person).collect();
    let told = if d <= y.rfa_deadline.add(10) { w.front_office.nfl.sheets_told.len() } else { 0 };
    for k in 0..told {
        let (id, y0) = w.front_office.nfl.sheets_told[k];
        if y0 == year && !sheets.iter().any(|&i| w.people[i].player.id == id) {
            let name = super::indices(w, &[id]).get(&id).map(|&i| w.people[i].name.clone());
            if let Some(name) = name {
                unpin(w, y.league_year.date, &format!("signed {name} to an offer sheet"));
            }
        }
    }
    // What happens on day d you read the next morning, so each reminder
    // goes out the day before the morning it speaks of.
    // The tag window opens: your expiring players.
    if d == y.tag_window_open.add(-1) && super::yours(w, t, Duty::Tags) && year >= 1993 {
        let n = expiring_count(w, t);
        if n > 0 {
            remind(
                w,
                1,
                format!(
                    "The tag window opens today: {n} of your players' deals run out at the new league year. Re-sign, tag or tender them by {}.",
                    y.tag_deadline.date.pretty()
                ),
                false,
            );
        }
    }
    if d == y.tag_deadline.date.add(-2) && super::yours(w, t, Duty::Tags) && year >= 1993 {
        let n = expiring_count(w, t);
        if n > 0 {
            remind(
                w,
                2,
                format!(
                    "The tag deadline is {}: {n} of your expiring players have no new deal, tag or tender.",
                    y.tag_deadline.date.pretty()
                ),
                true,
            );
        }
    }
    if d == y.fifth_year_option.add(-8) && super::yours(w, t, Duty::Contracts) {
        let open: Vec<String> = options_open(w, t);
        if !open.is_empty() {
            remind(
                w,
                3,
                format!(
                    "Fifth-year options are due {} ({}). Undecided options are declined.",
                    y.fifth_year_option.pretty(),
                    open.join(", ")
                ),
                true,
            );
        }
    }
    // Offer sheets other clubs signed with your restricted players.
    let mine: Vec<(usize, Date, Offer)> =
        w.market.sheets.iter().filter(|s| s.old == t).map(|s| (s.person, s.until, s.offer)).collect();
    for (i, until, o) in mine {
        let id = w.people[i].player.id;
        if w.front_office.nfl.sheets_told.contains(&(id, year)) {
            continue;
        }
        w.front_office.nfl.sheets_told.push((id, year));
        let name = w.people[i].name.clone();
        let text = format!(
            "The {} signed {name} to an offer sheet: {} {} at {} a year. You have until {} to match.",
            w.nfl[o.team].name,
            o.years,
            crate::world::folk::noun(o.years, "year"),
            super::dollars(o.apy),
            until.pretty()
        );
        super::tell_about(w, d, text, true, &[i]);
    }
    // Talks the market settled: signed with you, gone elsewhere.
    let mut done: Vec<(usize, TalkState, String)> = Vec::new();
    let ids: Vec<u32> = w
        .front_office
        .nfl
        .talks
        .iter()
        .filter(|x| matches!(x.state, TalkState::Standing | TalkState::Agreed | TalkState::Open | TalkState::Broken))
        .map(|x| x.player)
        .collect();
    let at = super::indices(w, &ids);
    for (k, talk) in w.front_office.nfl.talks.iter().enumerate() {
        if !matches!(talk.state, TalkState::Standing | TalkState::Agreed | TalkState::Open | TalkState::Broken) {
            continue;
        }
        let Some(&i) = at.get(&talk.player) else {
            done.push((k, TalkState::Gone, String::new()));
            continue;
        };
        let p = &w.people[i];
        let Some(c) = p.pro.as_ref() else { continue };
        let signed_here = c.team == Some(t) && c.contract.as_ref().is_some_and(|x| x.signed == year && x.team == t);
        match talk.kind {
            TalkKind::FreeAgent => {
                if signed_here && c.status == Status::Roster {
                    done.push((k, TalkState::Signed, format!("{} signed with you.", p.name)));
                } else if c.team.is_some_and(|o| o != t) && c.status == Status::Roster && d >= y.league_year.date {
                    let to = w.nfl[c.team.unwrap()].name;
                    done.push((k, TalkState::Gone, format!("{} signed with the {to}.", p.name)));
                } else if let Some(x) = w.market.pending.iter().find(|x| x.person == i && x.offer.team == t)
                    && talk.state == TalkState::Standing
                {
                    // In the negotiating window it's a verbal agreement
                    // that signs with the league year (real); on the open
                    // market he takes his week, a better offer can sway him.
                    let text = if d < y.league_year.date {
                        format!(
                            "{} agreed to terms with you: a verbal agreement that signs when the league year opens, {}.",
                            p.name,
                            y.league_year.date.pretty()
                        )
                    } else {
                        format!(
                            "{} agreed to your offer; it signs on {} unless a clearly better one sways him.",
                            p.name,
                            x.until.pretty()
                        )
                    };
                    done.push((k, TalkState::Agreed, text));
                }
            }
            TalkKind::ReSign => {
                if d >= y.league_year.date && c.team != Some(t) {
                    done.push((k, TalkState::Gone, String::new()));
                }
            }
            TalkKind::OfferSheet => {
                if c.team == Some(t) {
                    done.push((k, TalkState::Signed, format!("{}'s club didn't match: he's yours.", p.name)));
                } else if d > y.rfa_deadline.add(6) {
                    done.push((k, TalkState::Gone, String::new()));
                }
            }
            TalkKind::Extension => {}
        }
    }
    for (k, state, text) in done {
        let id = w.front_office.nfl.talks[k].player;
        let was = w.front_office.nfl.talks[k].state;
        w.front_office.nfl.talks[k].state = state;
        if !text.is_empty() && was != state {
            w.front_office.nfl.talks[k].lines.push(Line { date: d, ours: false, text: text.clone() });
            super::note(w, text.clone(), Some(id));
            let who: Vec<usize> = at.get(&id).copied().into_iter().collect();
            super::tell_about(w, d, text, false, &who);
        }
    }
    // Broken talks thaw.
    for talk in w.front_office.nfl.talks.iter_mut() {
        if talk.state == TalkState::Broken && talk.cold_until.is_some_and(|c| d >= c) {
            talk.state = TalkState::Open;
            talk.patience = 0.6;
        }
    }
    // Keep the talks list small: a year of closed talks.
    w.front_office
        .nfl
        .talks
        .retain(|x| !matches!(x.state, TalkState::Signed | TalkState::Gone) || x.opened.days_to(d) < 365);
    w.front_office.nfl.visits.retain(|v| v.1.days_to(d) < 365);
}

/// Your players whose deals run out with no new deal, tag or tender yet.
pub fn expiring_count(w: &World, t: usize) -> usize {
    let year = w.year;
    w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().and_then(|c| c.contract.as_ref()).is_some_and(|k| k.last_year() < year))
        .count()
}

fn options_open(w: &World, t: usize) -> Vec<String> {
    w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| {
            let id = w.people[i].player.id;
            w.people[i].pro.as_ref().is_some_and(|c| {
                c.draft.is_some_and(|d| d.1 == 1 && d.0 == w.year - 3)
                    && c.contract.as_ref().is_some_and(|k| k.kind == money::Kind::Rookie && k.option.is_none())
            }) && !w.front_office.nfl.options.iter().any(|x| x.0 == id)
        })
        .map(|&i| w.people[i].name.clone())
        .collect()
}
