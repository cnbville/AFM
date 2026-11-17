//! Deals you build, weighed by the game (lane N3; `docs/design/
//! live-talks-persona.md`, sections 8.2 and 11, question 3).
//!
//! You put a deal together from real things (`terms::Give`) and usage
//! terms (`usage::Usage`). The game values it against his best option and
//! answers yes, a counter, "let me talk to my mom", a lowball or no. His
//! counter is built by the game from what he wants most that the deal
//! doesn't give, sized to close the gap; the AI only says it.
//!
//! What a deal is worth comes from the systems that already price him:
//!
//! - **A pro** (`ProCareer`): his market price (`people::pay_and_worth`),
//!   asked up by greed and ego, down by his own discount for your club.
//!   His agent takes a day or two unless he negotiates alone (`goals`).
//! - **A recruit** (a high school player, or a transfer you recruit): each
//!   school's appeal on his own signing-day sums (`recruiting::appeal`),
//!   your money in it, your other gives as case points capped like a case
//!   (`case::CASE_TOP`). His best option is the school that tops your
//!   appeal. A family decider takes a few days.
//! - **Your own college player**: his lean to the portal (`cal.lean`).
//!   Each give pulls it down the way the portal meeting's promise already
//!   does (`promise::portal_pull`: 0.5 x weight x what it adds x belief),
//!   made general to every give. He agrees to stay when it falls under the
//!   lean that makes a man a leaver (0.25, as `talks.rs` reads it).
//! - **Anyone else**: his wants (lane N1's `Wanting`) weighed against what
//!   leaving would take.
//!
//! A promise in a deal is believed as P1 believes it (`promise::
//! belief_own`, `belief_out`, `usage::likely`); a thing done at once is
//! believed in full. Every size here is Claude's (proposed, not confirmed).

use super::Them;
use super::memory::{Agreed, Anchor, Owed};
use super::terms::{Best, Counter, DealKind, Give, Offer, Valuing, Weighed};
use super::wants::{Want, WantKind, Wanting};
use crate::calendar::Date;
use crate::player::{PlayingTime, Trait};
use crate::world::folk::claims::{Ear, Group};
use crate::world::folk::promise::{self, Terms};
use crate::world::folk::talks::{self, PromiseKind};
use crate::world::folk::usage::{self, Usage};
use crate::world::folk::{Team, case};
use crate::world::{Stage, World};

/// A college player leaning past this is a leaver (`talks.rs` reads the
/// same line).
pub const STAY_LEAN: f64 = 0.25;
/// A deal takes at most this share off his lean: three quarters (one
/// promise at most halves it, as built). Proposed, not confirmed.
pub const DEAL_CUT: f64 = 0.75;
/// A pro asks his price up by greed and ego, down by his own discount
/// for your club (`pro::market::discount`). Proposed, not confirmed.
pub const ASK_GREED: f64 = 0.12;
pub const ASK_EGO: f64 = 0.05;
/// Your college player staying calls an offer under this share of his
/// lowest number a lowball (negotiation item 5). Proposed, not confirmed.
pub const OWN_LOWBALL: f64 = 0.5;
/// An offer under his lowest number raises a staying college player's
/// portal lean the day you make it by this much x (1 - offer / lowest) x
/// his money goal (item 5): half his number at full money care adds
/// 0.075. Proposed, not confirmed.
pub const LOWBALL_LEAN: f64 = 0.15;
/// And takes this much x the same share off his mood, with pay its
/// reason. Proposed, not confirmed.
pub const LOWBALL_MOOD: f64 = 0.2;
/// A pro calls an offer under this share of his ask a lowball.
/// Proposed, not confirmed.
pub const LOWBALL: f64 = 0.75;
/// Your first number pulls his ask this share of the way to it, at most
/// a tenth of his ask (anchors, section 11). Proposed, not confirmed.
pub const ANCHOR: f64 = 0.1;
/// A recruit's give lands as a case point would: this much appeal at full
/// care and belief, all gives together at most `case::CASE_TOP`.
/// Proposed, not confirmed.
pub const GIVE_APPEAL: f64 = 0.02;
/// What a recruit needs over his best other school before he says yes,
/// in appeal points. Proposed, not confirmed.
pub const RECRUIT_MARGIN: f64 = 0.005;
/// A recruit whose money offer is under this share of his best school's
/// is lowballed, when money matters to his camp. Proposed, not confirmed.
pub const RECRUIT_LOWBALL: f64 = 0.5;
/// Days to talk it over: a parent or grandparent, a handler, an agent.
/// Proposed, not confirmed.
pub const FAMILY_DAYS: i32 = 3;
pub const HANDLER_DAYS: i32 = 5;
pub const AGENT_DAYS: i32 = 2;
/// A deadline inside this many days forces an answer now; it lowers his
/// bar a little for a modest man and raises it for a big ego (a squeeze).
/// Proposed, not confirmed.
pub const DEADLINE_DAYS: i32 = 7;
pub const DEADLINE_PULL: f64 = 0.04;
/// The same for a recruit, in appeal points per point of ego z.
/// Proposed, not confirmed.
pub const DEADLINE_APPEAL: f64 = 0.005;
/// Leaving takes this much of what he wants met (the generic case: a
/// coach with another job, anyone with somewhere to go). Proposed, not
/// confirmed.
pub const LEAVE_NEED: f64 = 0.4;
/// A rank's weight among his wants: the portal meeting's scale (1 first,
/// 0.6 second, 0.35 third, 0.15 after), made general.
pub const RANK_WEIGHT: [f64; 4] = [1.0, 0.6, 0.35, 0.15];

/// Weighs deals. Lane N1's `Wanting` gives his wants; with none (a person
/// it doesn't cover yet), a player's are read from his own stores.
pub struct Deals<'a> {
    pub wants: &'a dyn Wanting,
}

/// Who the deal is with, as the game prices him.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Who {
    /// A pro (person index), and whether he's yours already.
    Pro(usize, bool),
    /// A recruit or a transfer you recruit, with your program.
    Recruit(usize, usize),
    /// Your own player (college or high school).
    Own(usize),
    /// Anyone else.
    Other,
}

fn who(w: &World, him: Them) -> Who {
    let Them::Player(id) = him else { return Who::Other };
    let Some(i) = crate::world::folk::person(w, id) else { return Who::Other };
    let p = &w.people[i];
    let seat = Team::seat(w);
    let mine = seat.is_some_and(|t| t.roster(w).contains(&i));
    if p.pro.is_some() {
        return Who::Pro(i, mine);
    }
    match seat {
        Some(Team::College(j)) if !mine && (p.stage == Stage::HighSchool || p.cal.portal.is_some()) => {
            Who::Recruit(i, j)
        }
        _ if mine => Who::Own(i),
        _ => Who::Other,
    }
}

/// His wants, heaviest first: lane N1's, or his own stores.
fn wants_of(d: &Deals, w: &World, him: Them) -> Vec<Want> {
    let v = d.wants.wants(w, him);
    if !v.is_empty() {
        return v;
    }
    let Them::Player(id) = him else { return v };
    let Some(i) = crate::world::folk::person(w, id) else { return v };
    store_wants(w, i)
}

/// A player's wants from the stores that hold them: a pro's `goals`, a
/// recruit's decider's wants, anyone else's own.
fn store_wants(w: &World, i: usize) -> Vec<Want> {
    let p = &w.people[i];
    let mut v: Vec<(WantKind, f64)> = if p.pro.is_some() {
        let g = crate::world::pro::market::goals(w, i);
        vec![
            (WantKind::Money, g.money),
            (WantKind::Security, g.security),
            (WantKind::Winning, g.winning),
            (WantKind::Role, g.role),
            (WantKind::Place, g.home.max(g.city)),
            (WantKind::Loyalty, g.loyalty),
        ]
    } else {
        let ear = crate::world::folk::pulls::deciding_ear(w, i);
        let f = &p.family;
        let wn = if ear == Ear::Decider { &f.decider_wants } else { &f.own_wants };
        vec![
            (WantKind::Money, wn.money),
            (WantKind::Place, wn.close_to_home),
            (WantKind::Role, wn.playing_time),
            (WantKind::Academics, wn.academics),
            (WantKind::Development, wn.development),
            (WantKind::Winning, wn.prestige),
        ]
    };
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.into_iter().map(|(kind, x)| Want { kind, weight: x as f32, met: false }).collect()
}

/// A want's weight by its rank among his wants (`RANK_WEIGHT`); 0 when he
/// doesn't want it.
fn rank_weight(wants: &[Want], k: WantKind) -> f64 {
    wants.iter().position(|x| x.kind == k).map_or(0.0, |r| RANK_WEIGHT[r.min(3)])
}

/// How much a give meets each of his wants, 0-1 (before belief). `i` is
/// the man (for his own role and money), `money` his money a year to beat.
fn meets(w: &World, i: Option<usize>, give: &Give, money: f64) -> Vec<(WantKind, f64)> {
    let role_adds = |kind: PromiseKind| match i {
        Some(i) if w.people[i].pro.is_none() && w.people[i].stage != Stage::HighSchool => promise::adds(w, i, kind),
        _ => match kind {
            PromiseKind::Start => 0.8,
            _ => 0.5,
        },
    };
    let family = |id: u32| {
        i.is_some_and(|i| {
            w.folk
                .ties
                .of_id(w.people[i].player.id)
                .any(|t| t.kind == crate::world::folk::ties::Kind::Family && (t.a == id || t.b == id))
        })
    };
    match *give {
        Give::Scholarship(None) => vec![(WantKind::Academics, 0.5), (WantKind::Development, 0.4)],
        Give::Scholarship(Some(id)) if family(id) => vec![(WantKind::Family, 0.6)],
        Give::Scholarship(Some(_)) => vec![(WantKind::Friends, 0.5)],
        Give::Money(x) | Give::Outside(x) => vec![(WantKind::Money, (x / money.max(1.0)).clamp(0.0, 1.0))],
        Give::Years(y) => vec![(WantKind::Security, (y as f64 / 5.0).min(1.0))],
        Give::Guaranteed(g) => vec![(WantKind::Security, (g / (2.0 * money.max(1.0))).clamp(0.0, 1.0))],
        Give::Role(0) => vec![(WantKind::Role, role_adds(PromiseKind::Start))],
        Give::Role(1) => vec![(WantKind::Role, role_adds(PromiseKind::Snaps))],
        Give::Role(_) => vec![(WantKind::Role, 0.2)],
        Give::Position(_) => vec![(WantKind::Role, 0.25), (WantKind::Career, 0.15)],
        Give::NoRedshirt => vec![(WantKind::Role, 0.35)],
        Give::Jersey(_) => vec![(WantKind::Name, 0.1)],
        Give::EarlyEnrol => vec![(WantKind::Development, 0.2)],
        Give::TimeAway => vec![(WantKind::Family, 0.5)],
        Give::Tutor => vec![(WantKind::Academics, 0.4)],
        Give::StaffJob(_) => vec![(WantKind::Career, 0.7), (WantKind::Kids, 0.4)],
        Give::Council => vec![(WantKind::Career, 0.25), (WantKind::Name, 0.15)],
        Give::Stay => vec![(WantKind::Loyalty, 0.3), (WantKind::Development, 0.2)],
        Give::Pick(r) => vec![(WantKind::Club, (0.6 * (8.0 - r.clamp(1, 7) as f64) / 7.0).max(0.05))],
        Give::Favour => vec![(WantKind::Kids, 0.4), (WantKind::Name, 0.2)],
        // Lane N5b: the rest of a contract counts toward his money and his
        // security; a trade's parts toward a GM's club.
        Give::Bonus(x) => vec![(WantKind::Security, (x / (2.0 * money.max(1.0))).clamp(0.0, 1.0))],
        Give::Incentives(x) => vec![(WantKind::Money, (0.3 * x / money.max(1.0)).clamp(0.0, 1.0))],
        Give::VoidYears(_) | Give::Draft(_) | Give::PickIf(_) => vec![],
        Give::NoTrade => vec![(WantKind::Security, 0.3)],
        Give::Player(_) => vec![(WantKind::Club, 0.3)],
        Give::TheirPick(_) => vec![],
        Give::RevenueShare(x) => vec![(WantKind::Money, (x / money.max(1.0)).clamp(0.0, 1.0))],
        // A coach's title and the play-calling (lane N5c): his career and his
        // say in the scheme.
        Give::Title => vec![(WantKind::Career, 0.3), (WantKind::Name, 0.1)],
        Give::Calling => vec![(WantKind::Career, 0.4), (WantKind::Scheme, 0.6)],
    }
}

/// What a usage term meets: his role on the field, and his career (the
/// numbers it gets him).
fn term_meets() -> [(WantKind, f64); 2] {
    [(WantKind::Role, 0.35), (WantKind::Career, 0.15)]
}

/// The P1 promise a give stands for, when it is one.
fn promise_kind(g: &Give) -> Option<PromiseKind> {
    match *g {
        Give::Role(0) => Some(PromiseKind::Start),
        Give::Role(1) => Some(PromiseKind::Snaps),
        Give::Position(p) => Some(PromiseKind::Position(p)),
        Give::Stay => Some(PromiseKind::Stay),
        _ => None,
    }
}

/// How far he believes a give: in full when it's done at once, as P1
/// believes a promise otherwise.
fn belief(w: &World, who: Who, g: &Give) -> f64 {
    if !g.promised() {
        return 1.0;
    }
    match (who, promise_kind(g)) {
        (Who::Own(i), Some(k)) | (Who::Pro(i, true), Some(k)) => {
            promise::belief_own(w, i, k, &Terms::default(), 0).clamp(0.0, 1.0)
        }
        (Who::Recruit(i, j), _) => {
            let ear = crate::world::folk::pulls::deciding_ear(w, i);
            promise::belief_out(w, i, j, ear, 0.85).clamp(0.0, 1.0)
        }
        _ => 0.6,
    }
}

/// How far he believes a usage term.
fn term_belief(w: &World, who: Who, u: &Usage) -> f64 {
    match who {
        Who::Own(i) | Who::Pro(i, true) => {
            let t = Terms { usage: Some(*u), ..Terms::default() };
            promise::belief_own(w, i, PromiseKind::Usage, &t, 0).clamp(0.0, 1.0)
        }
        Who::Recruit(i, j) => {
            let ear = crate::world::folk::pulls::deciding_ear(w, i);
            promise::belief_out(w, i, j, ear, usage::likely(w, i, u)).clamp(0.0, 1.0)
        }
        Who::Pro(i, false) => usage::likely(w, i, u),
        Who::Other => 0.6,
    }
}

/// Each of his wants the offer meets, summed with belief and kept to 1:
/// (want, met).
fn met(w: &World, who: Who, offer: &Offer, money: f64) -> Vec<(WantKind, f64)> {
    let i = match who {
        Who::Pro(i, _) | Who::Recruit(i, _) | Who::Own(i) => Some(i),
        Who::Other => None,
    };
    let mut out: Vec<(WantKind, f64)> = Vec::new();
    let mut add = |k: WantKind, x: f64| match out.iter_mut().find(|y| y.0 == k) {
        Some(y) => y.1 = (y.1 + x).min(1.0),
        None => out.push((k, x.min(1.0))),
    };
    for g in &offer.gives {
        let b = belief(w, who, g);
        for (k, x) in meets(w, i, g, money) {
            add(k, x * b);
        }
    }
    for u in offer.terms.iter().filter(|u| !u.shown_only() && !u.season) {
        let b = term_belief(w, who, u);
        for (k, x) in term_meets() {
            add(k, x * b);
        }
    }
    out
}

/// The money a year he measures your number against.
fn money_bar(w: &World, who: Who) -> f64 {
    match who {
        Who::Pro(i, _) => crate::world::people::pay_and_worth(w, i).map_or(1.0, |x| x.1.max(x.0)),
        Who::Recruit(i, j) => crate::world::recruiting::usual_nil_offer(w, j, i).max(1.0),
        // One of yours: his market price (what a rival would pay to take
        // him) or what his school and collective paid him last July, the
        // higher; his own brand deals are not your money (Cain 2026-11-17).
        Who::Own(i) => {
            let market = crate::world::front_office::college::market_price(w, i).unwrap_or(0.0);
            market.max(w.people[i].nil_paid()).max(1.0)
        }
        Who::Other => 1.0,
    }
}

/// His closeness to you (coach and player bonds). 0 until the bonds build
/// is on main.
fn bond_with_you(_w: &World, _i: usize) -> f64 {
    0.0
}

/// How much your program wins, as a college player who wants to win reads
/// it: its last finished season's win share, 0 at .500 or worse, 1 at
/// .800, a conference title a fifth more (negotiation item 1, proposed,
/// not confirmed). 0 before a season is on the books.
fn program_contending(w: &World, j: usize) -> f64 {
    let label = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    w.programs[j].seasons.iter().rev().find(|s| s.year <= label && s.wins + s.losses > 0).map_or(0.0, |s| {
        let share = (s.wins as f64 / (s.wins + s.losses) as f64 - 0.5) / 0.3;
        (share + if s.conference_title { 0.2 } else { 0.0 }).clamp(0.0, 1.0)
    })
}

/// Your college player's discount for you (negotiation item 1, college),
/// on the NFL's parts and sizes (`pro::market::discount`): your bond, his
/// trust in you over half, your winning times his winning goal over half,
/// home (his home state's school, or within 250 miles) times his home goal;
/// the sum times his money factor and the money he has made (a
/// seven-figure year from his school and collective), at most
/// `FLOOR_MAX`. His goals are his own (`pro::market::goals`): a college
/// player answers for himself. Proposed, not confirmed.
pub fn college_discount(w: &World, i: usize) -> crate::world::pro::market::Discount {
    use crate::world::pro::market::{self as pm, Discount};
    let p = &w.people[i];
    let Some(j) = p.college.filter(|_| p.stage == Stage::College) else { return Discount::default() };
    let g = pm::goals(w, i);
    let bond = pm::FLOOR_BOND * bond_with_you(w, i);
    let trust = super::makeup::trust(w, Them::Player(p.player.id));
    let trust = pm::FLOOR_TRUST * ((trust - 0.5) / 0.5).clamp(0.0, 1.0);
    let win = pm::FLOOR_WIN * program_contending(w, j) * ((g.winning - 0.5) / 0.5).clamp(0.0, 1.0);
    let st = w.programs[j].state;
    let at_home = st == p.home_state || crate::world::geo::miles(p.home_state, st) <= 250.0;
    let home = if at_home { pm::FLOOR_HOME * g.home } else { 0.0 };
    let money = 1.6 - 1.2 * g.money;
    let made = (p.nil_paid() / 1.0e6).clamp(0.0, 1.0);
    let total = ((bond + trust + win + home) * money * (1.0 + pm::FLOOR_MADE * made)).clamp(0.0, pm::FLOOR_MAX);
    Discount { bond, trust, win, home, money, made, total }
}

/// Your college player's money numbers a year (negotiation items 5, 1 and
/// 3a), engine numbers never shown.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Numbers {
    /// His market (M): his market price, or last July's pay when higher.
    pub market: f64,
    /// His ask (A): his market moved by greed and ego, less his discount,
    /// pulled toward your first number.
    pub ask: f64,
    /// His lowest number for you (F): his market less his discount.
    pub lowest: f64,
    /// His bar (B): from his ask toward his lowest number by the true
    /// points made this season (and in this line, `argued`).
    pub bar: f64,
}

/// Your college player's numbers (`Numbers`). `argued` is the pull of the
/// points made in the line that carries the offer (`Offer::argued`).
pub fn own_numbers(w: &World, him: Them, i: usize, argued: f64) -> Numbers {
    let market = money_bar(w, Who::Own(i));
    let d = college_discount(w, i).total;
    let lowest = market * (1.0 - d);
    let ask = market * (1.0 + ASK_GREED * z(w, i, Trait::Greed) + ASK_EGO * z(w, i, Trait::Ego)) * (1.0 - d);
    let ask = anchored(w, him, ask).max(lowest);
    let pull = (super::negotiate::points::pull(w, w.people[i].player.id) + argued).clamp(0.0, 1.0);
    Numbers { market, ask, lowest, bar: lowest + (ask - lowest) * (1.0 - pull) }
}

/// Your college player's counter at his bar (negotiation items 5 and 3b).
/// Once a true performance point has landed this season and his bar
/// still sits above your offer, it comes back as a prove-it deal: a
/// smaller base (your number, at least his lowest) and the rest of his
/// bar as a clause on the stat his side wants paid (`points::earn_back`),
/// a year's deal. A man who wants security takes half the gap in his base;
/// a big ego wants all of it on the clause. Else his bar, in money.
fn own_counter(w: &World, i: usize, offer: &Offer, n: &Numbers) -> Counter {
    use super::negotiate::points;
    let bar = n.bar.round();
    let id = w.people[i].player.id;
    let x = offer_money(offer).unwrap_or(0.0);
    let back = points::named(w, id).and_then(|s| points::earn_back(w, i, s));
    if let Some(u) = back {
        let g = crate::world::pro::market::goals(w, i);
        let floor = x.max(n.lowest);
        let base = if g.security > 0.6 && z(w, i, Trait::Ego) < 0.5 { floor + 0.5 * (bar - floor) } else { floor };
        let base = base.round();
        let extra = (bar - base).max(0.0).round();
        if extra > 0.0 {
            let gives = vec![Give::Money(base), Give::Incentives(extra), Give::Years(1)];
            let mut asks: Vec<String> = gives.iter().map(|g| g.words()).collect();
            asks.push(format!("paid on {}", u.words()));
            return Counter { asks, gives, terms: vec![u], why: None };
        }
    }
    Counter { asks: vec![Give::Money(bar).words()], gives: vec![Give::Money(bar)], terms: vec![], why: None }
}

/// The money in an offer: its largest money give (NIL and revenue share
/// together, as both pots pay him).
fn offer_money(offer: &Offer) -> Option<f64> {
    let share: f64 =
        offer.gives.iter().filter_map(|g| if let Give::RevenueShare(x) = g { Some(*x) } else { None }).sum();
    offer
        .gives
        .iter()
        .filter_map(|g| match g {
            Give::Money(x) => Some(*x + share),
            Give::Outside(x) => Some(*x),
            _ => None,
        })
        .chain((share > 0.0).then_some(share))
        .reduce(f64::max)
}

/// His trait as a z score (-1 to 1).
fn z(w: &World, i: usize, t: Trait) -> f64 {
    w.people[i].player.personality.traits.z(t)
}

/// A deadline inside a week: the answer comes now.
fn pressing(w: &World, offer: &Offer) -> bool {
    offer.until.is_some_and(|d| w.today.days_to(d) <= DEADLINE_DAYS)
}

/// The bar's move under a pressing deadline: down for a modest man, up for
/// a big ego.
fn deadline_factor(w: &World, i: Option<usize>, offer: &Offer) -> f64 {
    match i {
        Some(i) if pressing(w, offer) => 1.0 + DEADLINE_PULL * z(w, i, Trait::Ego),
        _ => 1.0,
    }
}

/// When he'd answer if he takes time, and why: a family decider, a
/// handler, his agent. None when he decides himself or your deadline
/// presses.
fn takes_time(w: &World, who: Who, offer: &Offer) -> Option<Date> {
    if pressing(w, offer) {
        return None;
    }
    let days = match who {
        Who::Recruit(i, _) => match w.people[i].family.decider {
            crate::world::background::Decider::Himself => return None,
            crate::world::background::Decider::Handler => HANDLER_DAYS,
            _ => FAMILY_DAYS,
        },
        Who::Pro(i, _) => {
            let has_agent = w.people[i].pro.as_ref().is_some_and(|c| c.agent.is_some());
            if !has_agent || crate::world::pro::market::goals(w, i).alone {
                return None;
            }
            AGENT_DAYS
        }
        _ => return None,
    };
    let by = w.today.add(days);
    Some(offer.until.map_or(by, |u| by.min(u)))
}

/// The first number you put on the table with him this season.
fn anchor(w: &World, him: Them) -> Option<f64> {
    w.folk.live.persona.anchors.iter().find(|a| a.with == him && a.day.days_to(w.today) < 365).map(|a| a.money)
}

/// His ask moved by your first number (anchors).
fn anchored(w: &World, him: Them, ask: f64) -> f64 {
    match anchor(w, him) {
        Some(first) => {
            let pull = (ANCHOR * (first - ask)).clamp(-0.1 * ask, 0.1 * ask);
            ask + pull
        }
        None => ask,
    }
}

/// His usual usage ask at his position: the first measure that fits him
/// and is his to be given (not snaps, not a ceiling, not the other team's
/// call), at its usual number.
pub fn usual_term(w: &World, i: usize) -> Option<Usage> {
    let pos = w.people[i].player.position;
    let m = usage::MEASURES.iter().find(|m| {
        m.key != "snaps" && !m.most && m.spots.contains(&pos) && !usage::SHOWN.contains(&m.key) && m.key != "fg_line"
    })?;
    usage::read(pos, m.key, None, None, false, "any", 0, 0, 0, 0, 0)
}

/// His counter: what he wants most that the deal doesn't give, as a thing
/// the game can do, sized to close the gap. `money` the number he wants
/// when money is the want.
fn counter_for(w: &World, who: Who, wants: &[Want], offer: &Offer, money: Option<f64>) -> Option<Counter> {
    let have = met(w, who, offer, money_bar(w, who));
    let i = match who {
        Who::Pro(i, _) | Who::Recruit(i, _) | Who::Own(i) => Some(i),
        Who::Other => None,
    };
    for want in wants.iter().filter(|x| !x.met) {
        let got = have.iter().find(|x| x.0 == want.kind).map_or(0.0, |x| x.1);
        if got >= 0.6 && !(want.kind == WantKind::Money && money.is_some()) {
            continue;
        }
        let gives: Vec<Give> = match want.kind {
            WantKind::Money => vec![Give::Money(money.unwrap_or_else(|| money_bar(w, who) * 1.1))],
            WantKind::Role => {
                let starter = i.is_some_and(|i| match who {
                    Who::Own(_) => crate::world::folk::seen::starter(w, i),
                    _ => w.people[i].last_role == Some(PlayingTime::Starter),
                });
                if !starter && !offer.gives.contains(&Give::Role(0)) {
                    vec![Give::Role(0)]
                } else if offer.terms.is_empty() {
                    match i.and_then(|i| usual_term(w, i)) {
                        Some(u) => {
                            return Some(Counter {
                                asks: vec![format!("a promise of {}", u.words())],
                                gives: vec![],
                                terms: vec![u],
                                why: Some(*want),
                            });
                        }
                        None => continue,
                    }
                } else {
                    continue;
                }
            }
            WantKind::Security => vec![Give::Years(4), Give::Guaranteed(money_bar(w, who) * 2.0)],
            WantKind::Family => vec![Give::TimeAway],
            WantKind::Academics => vec![Give::Tutor],
            WantKind::Loyalty | WantKind::Development => vec![Give::Stay],
            WantKind::Career => vec![Give::StaffJob(None)],
            WantKind::Kids => vec![Give::Favour],
            WantKind::Name => vec![Give::Council],
            _ => continue,
        };
        let gives: Vec<Give> = gives.into_iter().filter(|g| !offer.gives.contains(g)).collect();
        if gives.is_empty() && want.kind != WantKind::Money {
            continue;
        }
        let asks = gives.iter().map(|g| g.words()).collect();
        return Some(Counter { asks, gives, terms: vec![], why: Some(*want) });
    }
    None
}

impl Deals<'_> {
    /// What he'd take, without taking time (the day he answers an owed
    /// deal, or a deadline that presses).
    pub fn weigh_now(&self, w: &World, him: Them, offer: &Offer) -> Weighed {
        self.weigh_in(w, him, offer, false)
    }

    fn weigh_in(&self, w: &World, him: Them, offer: &Offer, may_wait: bool) -> Weighed {
        let who = who(w, him);
        let wants = wants_of(self, w, him);
        let verdict = match who {
            Who::Pro(i, mine) => self.pro(w, him, i, mine, &wants, offer),
            Who::Recruit(i, j) => self.recruit(w, i, j, &wants, offer),
            Who::Own(i) => self.own(w, him, i, &wants, offer),
            Who::Other => self.other(w, him, &wants, offer),
        };
        match verdict {
            Weighed::Yes if may_wait => takes_time(w, who, offer).map_or(Weighed::Yes, Weighed::Later),
            v => v,
        }
    }

    /// A pro: your money against his ask.
    fn pro(&self, w: &World, him: Them, i: usize, mine: bool, wants: &[Want], offer: &Offer) -> Weighed {
        let worth = money_bar(w, Who::Pro(i, mine));
        // His own discount for your club (the negotiation overhaul, item
        // 1): the market's lowest number, the same the desk signs on.
        let home = match Team::seat(w) {
            Some(Team::Nfl(t)) => crate::world::pro::market::discount(w, i, t, mine).total,
            _ => 0.0,
        };
        let ask = worth * (1.0 + ASK_GREED * z(w, i, Trait::Greed) + ASK_EGO * z(w, i, Trait::Ego) - home);
        let ask = anchored(w, him, ask) * deadline_factor(w, Some(i), offer);
        let Some(apy) = offer_money(offer) else {
            // No money: a role or a term for a man under contract. He takes
            // it when it meets something he wants.
            return if met(w, Who::Pro(i, mine), offer, worth).iter().any(|x| rank_weight(wants, x.0) > 0.0) {
                Weighed::Yes
            } else {
                Weighed::No
            };
        };
        // Guarantees count for a man who wants security; the other gives
        // count at a tenth of his worth per want they meet in full.
        let security = rank_weight(wants, WantKind::Security);
        let gtd: f64 =
            offer.gives.iter().filter_map(|g| if let Give::Guaranteed(x) = g { Some(*x) } else { None }).sum();
        let years = offer.gives.iter().find_map(|g| if let Give::Years(y) = g { Some(*y as f64) } else { None });
        let rest: f64 = met(w, Who::Pro(i, mine), offer, worth)
            .iter()
            .filter(|x| !matches!(x.0, WantKind::Money | WantKind::Security))
            .map(|x| x.1 * rank_weight(wants, x.0))
            .sum();
        let value = apy + 0.15 * security * gtd / years.unwrap_or(1.0).max(1.0) + 0.1 * worth * rest;
        if value >= ask {
            Weighed::Yes
        } else if value < LOWBALL * ask {
            Weighed::Lowball
        } else {
            let mut c = counter_for(w, Who::Pro(i, mine), wants, offer, Some(ask.round())).unwrap_or(Counter {
                asks: vec![],
                gives: vec![],
                terms: vec![],
                why: None,
            });
            if !c.gives.iter().any(|g| matches!(g, Give::Money(_))) {
                c.gives.insert(0, Give::Money(ask.round()));
                c.asks.insert(0, Give::Money(ask).words());
            }
            Weighed::Counter(c)
        }
    }

    /// A recruit: your appeal with the deal against his best other school.
    fn recruit(&self, w: &World, i: usize, j: usize, wants: &[Want], offer: &Offer) -> Weighed {
        let who = Who::Recruit(i, j);
        let p = &w.people[i];
        if p.cal.signed.is_some_and(|s| s.program != j) {
            return Weighed::No;
        }
        let best = best_school(w, i, j);
        let lift = self.recruit_lift(w, who, offer);
        let yours = |money: Option<f64>| crate::world::recruiting::appeal(w, i, j, money) + lift;
        let money = offer_money(offer);
        let mine = yours(money);
        let Some((other, bar)) = best else { return Weighed::Yes };
        // A pressing deadline moves his bar: down for a modest kid, up for
        // a big ego.
        let squeeze = if pressing(w, offer) { DEADLINE_APPEAL * z(w, i, Trait::Ego) } else { 0.0 };
        let bar = bar + RECRUIT_MARGIN + squeeze;
        if mine >= bar {
            return Weighed::Yes;
        }
        // A money number far under his best school's, when money matters
        // to his camp.
        let money_care = crate::world::folk::claims::group_cares(p, crate::world::folk::pulls::deciding_ear(w, i))
            [Group::Money as usize];
        if let Some(x) = money {
            let theirs = crate::world::recruiting::offer_money(w, other, i);
            if money_care >= 1.0 && theirs > 0.0 && x < RECRUIT_LOWBALL * theirs {
                return Weighed::Lowball;
            }
        }
        // The money that would close it, up to the most the market would
        // pay him: the best top bid of the schools that want him, yours
        // included (the new NIL scheme, item 5: no going-rate top). Your
        // first number only ever raises what he asks: his number always
        // closes the gap.
        let top = p
            .cal
            .offers
            .iter()
            .chain(std::iter::once(&j))
            .map(|&k| crate::world::recruiting::recruit_top(w, k, i))
            .fold(0.0, f64::max);
        let base = money.unwrap_or_else(|| crate::world::recruiting::offer_money(w, j, i)).min(top);
        let need = if yours(Some(top)) >= bar && money_care > 0.0 {
            let (mut lo, mut hi) = (base, top);
            for _ in 0..24 {
                let mid = 0.5 * (lo + hi);
                if yours(Some(mid)) >= bar { hi = mid } else { lo = mid }
            }
            let ask = (hi / 1000.0).ceil() * 1000.0;
            Some(anchored(w, Them::Player(p.player.id), ask).max(ask).min(top.max(ask)))
        } else {
            None
        };
        match counter_for(w, who, wants, offer, need) {
            Some(c) if need.is_some() || c.gives.iter().all(|g| !matches!(g, Give::Money(_))) => Weighed::Counter(c),
            _ => Weighed::No,
        }
    }

    /// What a recruit's non-money gives add to your appeal: each as a case
    /// point by how much his camp cares, capped as a case is.
    fn recruit_lift(&self, w: &World, who: Who, offer: &Offer) -> f64 {
        let Who::Recruit(i, _) = who else { return 0.0 };
        let care = crate::world::folk::claims::group_cares(&w.people[i], crate::world::folk::pulls::deciding_ear(w, i));
        let group = |k: WantKind| match k {
            WantKind::Role => Some(Group::PlayingTime),
            WantKind::Academics | WantKind::Development => Some(Group::School),
            WantKind::Place => Some(Group::Home),
            WantKind::Family | WantKind::Friends => Some(Group::People),
            WantKind::Career | WantKind::Name => Some(Group::League),
            WantKind::Winning => Some(Group::Winning),
            _ => None,
        };
        let lift: f64 = met(w, who, offer, money_bar(w, who))
            .iter()
            .filter_map(|(k, x)| group(*k).map(|g| GIVE_APPEAL * care[g as usize] as f64 * x))
            .sum();
        lift.min(case::CASE_TOP as f64)
    }

    /// Your own player: what the deal takes off his lean to leave.
    ///
    /// A college player staying has his number (negotiation item 5): an
    /// offer at his bar is a yes, one under half his lowest number a
    /// lowball, and in between he counters at his bar (a prove-it shape
    /// once a performance point has landed, item 3b). Before, a player not
    /// thinking of leaving took any amount.
    fn own(&self, w: &World, him: Them, i: usize, wants: &[Want], offer: &Offer) -> Weighed {
        let lean = lean_of(w, i);
        let college = w.people[i].stage == Stage::College;
        if lean <= STAY_LEAN {
            if college && let Some(x) = offer_money(offer) {
                let n = own_numbers(w, him, i, offer.argued as f64);
                return if x >= n.bar {
                    Weighed::Yes
                } else if x < OWN_LOWBALL * n.lowest {
                    Weighed::Lowball
                } else {
                    Weighed::Counter(own_counter(w, i, offer, &n))
                };
            }
            // He isn't going anywhere: more is welcome, if it meets
            // anything he wants at all.
            return if met(w, Who::Own(i), offer, money_bar(w, Who::Own(i))).iter().any(|x| x.1 > 0.0) {
                Weighed::Yes
            } else {
                Weighed::No
            };
        }
        let after = lean * (1.0 - cut(w, i, wants, offer));
        if after <= STAY_LEAN {
            return Weighed::Yes;
        }
        if let Some(x) = offer_money(offer)
            && x < 0.5 * money_bar(w, Who::Own(i))
            && rank_weight(wants, WantKind::Money) >= 0.6
        {
            return Weighed::Lowball;
        }
        match counter_for(w, Who::Own(i), wants, offer, None) {
            Some(c) => {
                // Only a counter that would close the gap: else no.
                let mut more = offer.clone();
                more.gives.extend(c.gives.iter().cloned());
                more.terms.extend(c.terms.iter().copied());
                if lean * (1.0 - cut(w, i, wants, &more)) <= STAY_LEAN { Weighed::Counter(c) } else { Weighed::No }
            }
            None => Weighed::No,
        }
    }

    /// Anyone else: what the deal meets of his wants against what leaving
    /// would take.
    fn other(&self, w: &World, him: Them, wants: &[Want], offer: &Offer) -> Weighed {
        let gain: f64 =
            met(w, Who::Other, offer, 1.0).iter().map(|(k, x)| x * rank_weight(wants, *k)).sum::<f64>().min(1.0);
        let need = match self.best(w, him) {
            Best::Stay => 0.0,
            _ => LEAVE_NEED,
        };
        if gain > 0.0 && gain >= need {
            Weighed::Yes
        } else if gain < 0.3 * need {
            if offer.gives.is_empty() && offer.terms.is_empty() { Weighed::No } else { Weighed::Lowball }
        } else {
            counter_for(w, Who::Other, wants, offer, None).map_or(Weighed::No, Weighed::Counter)
        }
    }
}

/// The best other school on his list, with its appeal: the schools that
/// offered him, and his commitment's.
fn best_school(w: &World, i: usize, yours: usize) -> Option<(usize, f64)> {
    let p = &w.people[i];
    let mut list: Vec<usize> = p.cal.offers.iter().copied().filter(|&j| j != yours).collect();
    if let Some((j, _, _)) = p.cal.commit
        && j != yours
        && !list.contains(&j)
    {
        list.push(j);
    }
    if list.is_empty() {
        return None;
    }
    let a = crate::world::recruiting::appeals(w, i, &list, None);
    list.into_iter().zip(a).max_by(|x, y| x.1.total_cmp(&y.1))
}

/// Your own player's lean to leave if he took the deal (an engine number,
/// never shown): None for anyone else.
pub fn lean_after(w: &World, deals: &Deals, him: Them, offer: &Offer) -> Option<f64> {
    let Who::Own(i) = who(w, him) else { return None };
    let wants = wants_of(deals, w, him);
    Some(lean_of(w, i) * (1.0 - cut(w, i, &wants, offer)))
}

/// His lean to leave: in the portal already, nearly gone.
fn lean_of(w: &World, i: usize) -> f64 {
    let p = &w.people[i];
    if p.cal.portal.is_some() { p.cal.lean.max(0.9) } else { p.cal.lean.max(0.0) }
}

/// The share of his lean a deal takes off: each want it meets pulls by
/// 0.5 x the want's rank weight x what it meets (belief in it), as the
/// portal meeting's promise pulls; all together at most `DEAL_CUT`.
fn cut(w: &World, i: usize, wants: &[Want], offer: &Offer) -> f64 {
    let keep: f64 = met(w, Who::Own(i), offer, money_bar(w, Who::Own(i)))
        .iter()
        .map(|(k, x)| 1.0 - (promise::PULL_CAP * rank_weight(wants, *k) * x).clamp(0.0, promise::PULL_CAP))
        .product();
    (1.0 - keep).min(DEAL_CUT)
}

impl Valuing for Deals<'_> {
    fn best(&self, w: &World, him: Them) -> Best {
        match who(w, him) {
            Who::Pro(i, _) => {
                let c = w.people[i].pro.as_ref();
                let left = c.and_then(|c| c.contract.as_ref()).map_or(0, |k| k.last_year() - w.year);
                if left <= 0 { Best::FreeAgency } else { Best::Stay }
            }
            Who::Recruit(i, j) => best_school(w, i, j).map_or(Best::Stay, |x| Best::School(x.0)),
            Who::Own(i) => {
                if lean_of(w, i) > STAY_LEAN {
                    Best::Portal
                } else {
                    Best::Stay
                }
            }
            Who::Other => match him {
                Them::Coach(c) if w.coaches.get(c).is_some_and(|x| x.job.is_none() && !x.retired) => {
                    Best::Other("another job".into())
                }
                _ => Best::Stay,
            },
        }
    }

    fn weigh(&self, w: &World, him: Them, offer: &Offer) -> Weighed {
        self.weigh_in(w, him, offer, true)
    }
}

// ---------------------------------------------------------------------------
// What a talk does with an offer (lane N4 calls these when it applies a
// turn).

/// What an offer cost you in his eyes: a lowball costs warmth, asking again
/// after a no costs patience (section 11). Signed, for N4 to apply inside
/// its caps. Proposed, not confirmed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Cost {
    pub trust: f32,
    pub patience: f32,
}

/// The cost of a lowball and of asking again (`again` the noes so far in
/// this talk).
pub fn cost(w: &World, him: Them, weighed: &Weighed, again: u8) -> Cost {
    let traits = match him {
        Them::Player(id) => crate::world::folk::person(w, id).map(|i| w.people[i].player.personality.traits),
        _ => None,
    };
    let ego = traits.map_or(0.0, |t| t.z(Trait::Ego).max(0.0));
    let temper = traits.map_or(0.0, |t| t.z(Trait::Temper).max(0.0));
    let mut c = Cost::default();
    if matches!(weighed, Weighed::Lowball) {
        c.trust -= (0.02 + 0.02 * ego + 0.01 * temper) as f32;
    }
    if again > 0 {
        c.patience -= (0.15 * again as f64 * (1.0 + 0.5 * temper)) as f32;
    }
    c
}

/// An offer under a staying college player's lowest number upsets him the
/// day you make it (negotiation item 5), not on July 1: his portal lean
/// rises by `LOWBALL_LEAN` and his mood falls by `LOWBALL_MOOD`, each x
/// (1 - offer / lowest) x his money goal, pay the mood's reason. A lowball's
/// trust cost is the talk's (`cost`); an offer under his number that isn't
/// one costs the same trust here. Nothing for an offer he took.
fn upset_on_the_day(w: &mut World, him: Them, offer: &Offer, weighed: &Weighed) {
    if matches!(weighed, Weighed::Yes) {
        return;
    }
    let Who::Own(i) = who(w, him) else { return };
    if w.people[i].stage != Stage::College || lean_of(w, i) > STAY_LEAN {
        return;
    }
    let Some(x) = offer_money(offer) else { return };
    // The line's points are kept by now (`talk::keep_points` runs first).
    let n = own_numbers(w, him, i, 0.0);
    if n.lowest <= 0.0 || x >= n.lowest {
        return;
    }
    let short = 1.0 - x / n.lowest;
    let care = crate::world::pro::market::goals(w, i).money;
    if !matches!(weighed, Weighed::Lowball) {
        let c = cost(w, him, &Weighed::Lowball, 0);
        super::stores::nudge_trust(w, him, c.trust);
    }
    let p = &mut w.people[i];
    p.cal.lean = (p.cal.lean + LOWBALL_LEAN * short * care).min(1.0);
    p.mood = (p.mood - (LOWBALL_MOOD * short * care) as f32).max(-1.0);
    if p.mood < -0.15 {
        p.mood_why = crate::world::people::Why::Money as u8;
    }
}

/// Keeps what an offer leaves behind: your first number (an anchor), and
/// an answer he owes you when he took time.
pub fn offered(w: &mut World, him: Them, offer: &Offer, weighed: &Weighed) {
    upset_on_the_day(w, him, offer, weighed);
    let today = w.today;
    let m = &mut w.folk.live.persona;
    if let Some(x) = offer_money(offer) {
        m.anchors.retain(|a| a.day.days_to(today) < 365);
        if !m.anchors.iter().any(|a| a.with == him) {
            m.anchors.push(Anchor { with: him, day: today, money: x });
        }
    }
    if let Weighed::Later(by) = weighed {
        m.owed.retain(|o| o.with != him);
        m.owed.push(Owed { with: him, made: today, by: *by, offer: offer.clone() });
    }
}

/// The answers owed today: each weighed again now, with his decider's or
/// his agent's say had, and taken off the list.
pub fn due(w: &mut World, deals: &Deals) -> Vec<(Owed, Weighed)> {
    let today = w.today;
    let (now, later): (Vec<Owed>, Vec<Owed>) =
        std::mem::take(&mut w.folk.live.persona.owed).into_iter().partition(|o| o.by <= today);
    w.folk.live.persona.owed = later;
    now.into_iter()
        .map(|o| {
            let v = deals.weigh_now(w, o.with, &o.offer);
            (o, v)
        })
        .collect()
}

/// The answers owed today to the people `mine` picks, the rest left on
/// the list: each facet lane takes its own (lane N5a: recruits and their
/// families), so no lane answers for another's.
pub fn due_where(w: &mut World, deals: &Deals, mine: impl Fn(&World, Them) -> bool) -> Vec<(Owed, Weighed)> {
    let today = w.today;
    let all = std::mem::take(&mut w.folk.live.persona.owed);
    let (now, later): (Vec<Owed>, Vec<Owed>) = all.into_iter().partition(|o| o.by <= today && mine(w, o.with));
    w.folk.live.persona.owed = later;
    now.into_iter()
        .map(|o| {
            let v = deals.weigh_now(w, o.with, &o.offer);
            (o, v)
        })
        .collect()
}

/// Whether he is a recruit or a transfer you recruit, as deals price him.
pub fn is_recruit(w: &World, him: Them) -> bool {
    matches!(who(w, him), Who::Recruit(..))
}

/// What agreeing did, after you confirmed: what the game made real, the
/// promises logged (P1's indexes), and the gives its facet lane makes
/// real (money, a scholarship, a contract, a pick).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Agreement {
    pub done: Vec<String>,
    pub promised: Vec<u32>,
    pub left: Vec<Give>,
    /// Usage terms a recruit's pledge or a pro's contract carries.
    pub left_terms: Vec<Usage>,
}

/// Makes an agreed deal real: its promises through P1 (a usage term as a
/// usage promise), his lean for your own college player, time away and a
/// council seat at once, and the deal kept in his memory with your words.
pub fn agree(w: &mut World, deals: &Deals, him: Them, offer: &Offer) -> Agreement {
    let mut out = Agreement::default();
    let whom = who(w, him);
    if let Who::Own(i) = whom
        && w.people[i].stage == Stage::College
        && w.people[i].cal.lean > 0.0
    {
        let wants = wants_of(deals, w, him);
        let k = 1.0 - cut(w, i, &wants, offer);
        w.people[i].cal.lean *= k;
        out.done.push("He's staying: his mind is off the portal.".into());
    }
    let to = him.promise_to();
    let name = match him {
        Them::Player(id) => crate::world::folk::person(w, id).map(|i| w.people[i].name.clone()),
        _ => None,
    }
    .unwrap_or_else(|| "He".into());
    for g in &offer.gives {
        match (g, promise_kind(g), to) {
            (_, Some(kind), Some(to)) if matches!(whom, Who::Own(_) | Who::Pro(_, true)) => {
                talks::promise(w, to, kind, 0);
                out.promised.push((w.folk.promises.len() - 1) as u32);
                out.done.push(format!("You promised {name} {}.", kind.words()));
            }
            (Give::TimeAway, _, Some(talks::PromiseTo::Player(id))) => {
                crate::world::folk::life::grant_leave(w, id);
                out.done.push(format!("{name} gets time away."));
            }
            (Give::Council, _, Some(talks::PromiseTo::Player(id))) => {
                if !w.folk.rules.council.contains(&id) {
                    w.folk.rules.council.push(id);
                }
                out.done.push(format!("{name} joins your leadership council."));
            }
            (Give::Tutor, _, Some(_)) => out.done.push(format!("{name} gets a tutor.")),
            (Give::Jersey(n), _, Some(_)) => out.done.push(format!("{name} wears number {n}.")),
            _ => out.left.push(g.clone()),
        }
    }
    if let Some(to @ talks::PromiseTo::Player(_)) = to
        && matches!(whom, Who::Own(_) | Who::Pro(_, true))
    {
        // A season term (an incentive's, a clause's) is never a promise:
        // it rides on the deal its facet lane signs.
        out.left_terms.extend(offer.terms.iter().filter(|u| u.season).copied());
        for u in offer.terms.iter().filter(|u| !u.shown_only() && !u.season) {
            talks::promise(w, to, PromiseKind::Usage, 0);
            let k = w.folk.promises.len() - 1;
            talks::set_terms(w, k, Terms { usage: Some(*u), ..Terms::default() });
            out.promised.push(k as u32);
            out.done.push(format!("You promised {name} {}.", u.words()));
        }
    } else {
        // A recruit's or another club's pro's usage terms ride on the deal
        // its facet lane signs (a recruit's pledge, a contract's clause).
        out.left_terms.extend(offer.terms.iter().copied());
    }
    let today = w.today;
    let m = &mut w.folk.live.persona;
    m.owed.retain(|o| o.with != him);
    m.agreed.push(Agreed { with: him, day: today, offer: offer.clone() });
    out
}

/// What kind a deal is, from what's in it, when your words didn't say.
pub fn kind_of(w: &World, him: Them, offer: &Offer) -> DealKind {
    if offer.kind != DealKind::Promise {
        return offer.kind;
    }
    let pro = matches!(who(w, him), Who::Pro(..));
    if offer.gives.iter().any(|g| matches!(g, Give::Outside(_))) {
        DealKind::Outside
    } else if offer.gives.iter().any(|g| matches!(g, Give::Pick(_) | Give::TheirPick(_) | Give::Player(_))) {
        DealKind::Trade
    } else if pro
        && offer.gives.iter().any(|g| {
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
    {
        DealKind::Contract
    } else if offer.gives.iter().any(|g| matches!(g, Give::Money(_) | Give::RevenueShare(_))) {
        DealKind::Nil
    } else if offer.gives.iter().any(|g| matches!(g, Give::Scholarship(None))) {
        DealKind::Scholarship
    } else if offer.gives.iter().any(|g| matches!(g, Give::StaffJob(_))) {
        DealKind::Staff
    } else if offer.gives.iter().any(|g| matches!(g, Give::Favour)) {
        DealKind::Favour
    } else {
        DealKind::Promise
    }
}

/// Whether your player agreed a money deal with you in a talk since
/// `since` (negotiation item 5: no second upset on July 1).
pub fn agreed_money(w: &World, id: u32, since: Date) -> bool {
    w.folk.live.persona.agreed.iter().any(|a| {
        a.with == Them::Player(id)
            && a.day >= since
            && a.offer.gives.iter().any(|g| matches!(g, Give::Money(_) | Give::RevenueShare(_)))
    })
}
