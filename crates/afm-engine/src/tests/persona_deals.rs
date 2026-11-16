//! Live talks v3, lane N3: deals and requests (`world::folk::persona::deals`
//! and `asks`). The proofs: a deal on his top want takes more off a
//! leaver's lean than one on his fourth, his counter is what he wants most
//! that the deal doesn't give, and agreeing makes it real (his lean, P1's
//! promises, his memory); a pro's ask comes from his market price, his
//! greed and your first number, with a lowball under it; a recruit's
//! counter is the money that tops his best other school, and a family
//! decider takes days unless your deadline presses; a mentor ask makes a
//! real tie and a threatened starter balks; a teammate talk moves the
//! teammate's mood and their friendship the same way; a friend's pitch is a
//! real point in your case; and what asks you to confirm.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::positions::Position;
use crate::world::folk::claims::Group;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::asks::{self, Asks};
use crate::world::folk::persona::deals::{self, Deals};
use crate::world::folk::persona::ear::{Act, Confirm};
use crate::world::folk::persona::fake::Fake;
use crate::world::folk::persona::memory::Secret;
use crate::world::folk::persona::terms::{Best, Give, Offer, Request, Valuing, Weighed};
use crate::world::folk::persona::wants::{Acting, Want, WantKind, Willing};
use crate::world::folk::{case, person, ties};
use crate::world::{Stage, World};

fn wants(kinds: &[WantKind]) -> Vec<Want> {
    kinds.iter().enumerate().map(|(k, &kind)| Want { kind, weight: 1.0 - 0.2 * k as f32, met: false }).collect()
}

/// A college world with your program, and a class to draw people from.
fn college() -> (World, usize) {
    let mut w = seat("college");
    w.add_class_aged(19);
    w.add_class_aged(17);
    let p = w.seat.and_then(|s| s.program).unwrap();
    (w, p)
}

/// The next high schooler not used yet.
fn fresh(w: &World, skip: &[usize], age: u32) -> usize {
    (0..w.people.len())
        .find(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].player.age == age && !skip.contains(&i))
        .or_else(|| (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool && !skip.contains(&i)))
        .unwrap()
}

/// One of your college players: a backup with this lean to leave, who
/// trusts you.
fn own(w: &mut World, p: usize, skip: &[usize], lean: f64) -> usize {
    let i = fresh(w, skip, 19);
    let you = w.career.as_ref().map(|c| c.coach).unwrap() as u32;
    let year = w.year;
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 1;
    q.college_seasons = 1;
    q.last_role = Some(PlayingTime::Backup);
    q.trust = 0.8;
    q.trust_in = you;
    q.cal.lean = lean;
    w.programs[p].roster.push(i);
    i
}

#[test]
fn a_deal_on_his_top_want_keeps_a_leaver_and_agreeing_makes_it_real() {
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], 0.3);
    let id = w.people[i].player.id;
    let him = Them::Player(id);
    let f = Fake {
        wants: wants(&[WantKind::Role, WantKind::Money, WantKind::Place, WantKind::Academics]),
        ..Default::default()
    };
    let d = Deals { wants: &f };
    assert_eq!(d.best(&w, him), Best::Portal);
    let start = Offer { gives: vec![Give::Role(0)], ..Default::default() };
    let tutor = Offer { gives: vec![Give::Tutor], ..Default::default() };
    let a = deals::lean_after(&w, &d, him, &start).unwrap();
    let b = deals::lean_after(&w, &d, him, &tutor).unwrap();
    assert!(a < b && b < 0.3, "a start (his top want) {a:.3} against a tutor (his fourth) {b:.3}");
    // The tutor alone doesn't keep him: he counters with what he wants
    // most that it doesn't give, a start, which would.
    match d.weigh(&w, him, &tutor) {
        Weighed::Counter(c) => {
            assert_eq!(c.why.map(|x| x.kind), Some(WantKind::Role));
            assert_eq!(c.gives, vec![Give::Role(0)]);
        }
        v => panic!("expected a counter, got {v:?}"),
    }
    // Nothing at all on the table: he says what would keep him.
    assert!(matches!(d.weigh(&w, him, &Offer::default()), Weighed::Counter(c) if c.gives == vec![Give::Role(0)]));
    // The start with a usage term at his spot keeps him. Agreeing lowers
    // his lean, logs both promises through P1 and keeps the deal.
    w.people[i].player.position = Position::RB;
    let term = deals::usual_term(&w, i).unwrap();
    let deal = Offer { gives: vec![Give::Role(0), Give::Tutor], terms: vec![term], ..Default::default() };
    assert_eq!(d.weigh(&w, him, &deal), Weighed::Yes);
    let before = w.folk.promises.len();
    let done = deals::agree(&mut w, &d, him, &deal);
    assert!(w.people[i].cal.lean <= deals::STAY_LEAN, "lean {}", w.people[i].cal.lean);
    assert_eq!(done.promised.len(), 2);
    assert_eq!(w.folk.promises.len(), before + 2);
    assert!(w.folk.promises[before + 1].terms.usage == Some(term));
    assert_eq!(w.folk.live.persona.agreed.len(), 1);
    assert!(done.left.is_empty(), "a tutor is done at once: {:?}", done.left);
}

/// An NFL world and one of your club's veterans with a market price.
fn pro() -> (World, usize, f64) {
    let w = seat("nfl");
    let club = w.seat.and_then(|s| s.nfl_team).unwrap();
    let i = w.clubs[club]
        .roster
        .iter()
        .copied()
        .find(|&i| crate::world::people::pay_and_worth(&w, i).is_some_and(|(pay, worth)| worth > pay && worth > 1.0e6))
        .or_else(|| {
            w.clubs[club].roster.iter().copied().find(|&i| crate::world::people::pay_and_worth(&w, i).is_some())
        })
        .unwrap();
    let (pay, worth) = crate::world::people::pay_and_worth(&w, i).unwrap();
    (w, i, worth.max(pay))
}

#[test]
fn a_pro_asks_his_market_price_moved_by_greed_and_your_first_number() {
    let (mut w, i, worth) = pro();
    let id = w.people[i].player.id;
    let him = Them::Player(id);
    {
        let t = &mut w.people[i].player.personality.traits;
        (t.greed, t.ego, t.loyalty) = (20, 10, 10);
    }
    let f = Fake { wants: wants(&[WantKind::Money, WantKind::Role]), ..Default::default() };
    let d = Deals { wants: &f };
    let money = |x: f64| Offer { gives: vec![Give::Money(x)], ..Default::default() };
    let ask = worth * (1.0 + deals::ASK_GREED);
    // Over his ask: yes (his agent may take a day or two); with your
    // deadline this week, yes now.
    let rich = money(ask * 1.05);
    assert!(matches!(d.weigh(&w, him, &rich), Weighed::Yes | Weighed::Later(_)));
    let pressed = Offer { until: Some(w.today.add(3)), ..rich.clone() };
    assert_eq!(d.weigh(&w, him, &pressed), Weighed::Yes);
    // Far under: a lowball, which costs warmth.
    let low = d.weigh(&w, him, &money(ask * 0.5));
    assert_eq!(low, Weighed::Lowball);
    assert!(deals::cost(&w, him, &low, 0).trust < 0.0);
    // In between: his counter is his ask.
    let near = money(ask * 0.92);
    let counter_money = |v: Weighed| match v {
        Weighed::Counter(c) => {
            c.gives.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None }).unwrap()
        }
        v => panic!("expected a counter, got {v:?}"),
    };
    let greedy = counter_money(d.weigh(&w, him, &near));
    assert!((greedy - ask).abs() <= 1.0, "counter {greedy} against ask {ask}");
    // A modest man asks less.
    w.people[i].player.personality.traits.greed = 4;
    let modest = counter_money(d.weigh(&w, him, &money(ask * 0.8)));
    assert!(modest < greedy);
    w.people[i].player.personality.traits.greed = 20;
    // Your first number pulls his ask toward it, a little.
    deals::offered(&mut w, him, &near, &Weighed::No);
    let anchored = counter_money(d.weigh(&w, him, &near));
    assert!(anchored < greedy && anchored > 0.9 * greedy, "{anchored} against {greedy}");
}

#[test]
fn a_recruit_counters_with_the_money_that_tops_his_best_school_and_his_mom_takes_days() {
    let (mut w, p) = college();
    let other = (0..w.programs.len()).find(|&j| j != p && w.programs[j].division == w.programs[p].division).unwrap();
    let f = Fake { wants: wants(&[WantKind::Money, WantKind::Place]), ..Default::default() };
    let d = Deals { wants: &f };
    let small = Offer { gives: vec![Give::Scholarship(None), Give::Money(1000.0)], ..Default::default() };
    // A rich program of yours: the most he can ask is the best top bid for
    // him, and only money your budget can reach closes the gap (the new NIL
    // scheme, item 5).
    w.programs[p].collective.budget = 40.0e6;
    let mut seen = 0;
    let mut used = Vec::new();
    for _ in 0..200 {
        let r = fresh(&w, &used, 17);
        used.push(r);
        w.people[r].cal.offers = vec![other];
        // A five-star, whom the market pays real money: the most he can ask
        // is the best top bid for him (the new NIL scheme, item 5).
        w.people[r].stars = 5;
        w.people[r].family.decider = crate::world::background::Decider::Himself;
        let him = Them::Player(w.people[r].player.id);
        assert_eq!(d.best(&w, him), Best::School(other));
        let Weighed::Counter(c) = d.weigh(&w, him, &small) else { continue };
        let Some(x) = c.gives.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None }) else {
            continue;
        };
        // His number is enough.
        let enough = Offer { gives: vec![Give::Scholarship(None), Give::Money(x)], ..Default::default() };
        assert_eq!(d.weigh(&w, him, &enough), Weighed::Yes, "recruit {r}: his own number {x}");
        // With his mother deciding, he takes days; your deadline this week
        // makes him answer now.
        w.people[r].family.decider = crate::world::background::Decider::Parent;
        assert_eq!(d.weigh(&w, him, &enough), Weighed::Later(w.today.add(deals::FAMILY_DAYS)));
        let pressed = Offer { until: Some(w.today.add(2)), ..enough.clone() };
        assert_eq!(d.weigh(&w, him, &pressed), Weighed::Yes);
        // The answer he owes comes due on its day.
        let by = w.today.add(deals::FAMILY_DAYS);
        deals::offered(&mut w, him, &enough, &Weighed::Later(by));
        assert!(deals::due(&mut w, &d).is_empty());
        w.today = w.today.add(deals::FAMILY_DAYS);
        let due = deals::due(&mut w, &d);
        assert_eq!(due.len(), 1);
        assert!(w.folk.live.persona.owed.is_empty());
        w.today = w.today.add(-deals::FAMILY_DAYS);
        seen += 1;
        break;
    }
    assert!(seen > 0, "no recruit countered with money");
}

#[test]
fn a_mentor_ask_makes_a_real_tie_and_a_threatened_starter_balks() {
    let (mut w, p) = college();
    let vet = own(&mut w, p, &[], 0.0);
    let kid = fresh(&w, &[vet], 17);
    let (v, k) = (w.people[vet].player.id, w.people[kid].player.id);
    w.people[vet].player.age = 22;
    w.people[kid].player.age = 18;
    {
        let t = &mut w.people[vet].player.personality.traits;
        (t.leadership, t.professionalism, t.ego) = (18, 15, 8);
    }
    let ask = Request::Mentor(k);
    assert_eq!(Asks.willing(&w, Them::Player(v), &ask), Willing::Yes);
    let ran = Asks.run(&mut w, Them::Player(v), &ask);
    assert_eq!(ties::mentor_of(&w, k), Some(v));
    assert!(!ran.done.is_empty() && ran.honest);
    // The same man, the starter at the kid's spot, with a big ego and the
    // kid close behind him: he won't teach his replacement for nothing.
    w.people[kid].player.position = w.people[vet].player.position;
    w.people[vet].last_role = Some(PlayingTime::Starter);
    let o = w.people[vet].player.overall();
    let kid_o = w.people[kid].player.overall();
    if kid_o < o - 5.0 {
        // Make the kid a threat.
        w.people[vet].player.attributes = w.people[kid].player.attributes;
    }
    {
        let t = &mut w.people[vet].player.personality.traits;
        (t.leadership, t.ego) = (10, 20);
    }
    w.people[vet].trust = 0.5;
    assert_ne!(Asks.willing(&w, Them::Player(v), &ask), Willing::Yes);
}

#[test]
fn a_teammate_talk_moves_his_mood_and_their_friendship_the_same_way() {
    let (mut w, p) = college();
    let a = own(&mut w, p, &[], 0.0);
    let b = own(&mut w, p, &[a], 0.0);
    let (ia, ib) = (w.people[a].player.id, w.people[b].player.id);
    ties::bond(&mut w, ia, ib, 30);
    let (mood, tie) = (w.people[b].mood, ties::between(&w, ia, ib));
    let ran = Asks.run(&mut w, Them::Player(ia), &Request::TalkTo(ib));
    let (mood2, tie2) = (w.people[b].mood, ties::between(&w, ia, ib));
    assert_ne!(mood, mood2);
    assert_eq!(mood2 > mood, tie2 > tie, "mood {mood} -> {mood2}, tie {tie} -> {tie2}");
    assert_eq!(ran.report.len(), 1);
    // A talk that went well is always reported straight.
    if mood2 > mood {
        assert!(ran.honest);
    }
}

#[test]
fn a_friend_recruiting_a_recruit_is_a_point_in_your_case() {
    let (mut w, p) = college();
    let a = own(&mut w, p, &[], 0.0);
    let r = fresh(&w, &[a], 17);
    let (ia, ir) = (w.people[a].player.id, w.people[r].player.id);
    w.people[a].mood = 0.5;
    w.people[a].player.personality.traits.leadership = 16;
    w.people[a].hs = w.people[r].hs;
    ties::bond(&mut w, ia, ir, 40);
    let ask = Request::Recruit(ir);
    assert_eq!(Asks.willing(&w, Them::Player(ia), &ask), Willing::Yes);
    Asks.run(&mut w, Them::Player(ia), &ask);
    let c = case::of(&w.people[r], p).expect("a case with your school");
    assert!(c.has(Group::People));
    assert!(c.total() > 0.0, "a friend who's happy here helps: {}", c.total());
    // An unhappy man won't sell your school.
    w.people[a].mood = -0.9;
    w.people[a].trust = 0.2;
    assert_eq!(Asks.willing(&w, Them::Player(ia), &ask), Willing::No);
}

#[test]
fn keep_quiet_and_tell_me_are_kept() {
    let (mut w, p) = college();
    let a = own(&mut w, p, &[], 0.0);
    let ia = w.people[a].player.id;
    let him = Them::Player(ia);
    let you = Them::Coach(w.career.as_ref().unwrap().coach);
    w.folk.live.persona.secrets.push(Secret {
        by: you,
        yours: true,
        to: him,
        day: w.today,
        words: "I might take the Texas job".into(),
        asked_quiet: false,
        out: false,
    });
    Asks.run(&mut w, him, &Request::KeepQuiet);
    assert!(w.folk.live.persona.secrets[0].asked_quiet);
    let odds = asks::passes_on(&w, him);
    assert!((0.03..=0.8).contains(&odds));
    Asks.run(&mut w, him, &Request::TellMe(None));
    assert_eq!(w.folk.live.persona.watch.len(), 1);
    // The same break gives the same answer every time (a reload plays the
    // same world).
    let x = asks::tells_you(&w, 99, &[ia], w.today);
    assert_eq!(x, asks::tells_you(&w, 99, &[ia], w.today));
    assert_eq!(asks::tells_you(&w, 99, &[ia + 1], w.today), None);
    assert!(person(&w, ia).is_some());
}

#[test]
fn offers_decisions_and_costly_asks_wait_for_your_confirm() {
    let offer = Offer { gives: vec![Give::Role(0)], ..Default::default() };
    assert!(matches!(asks::confirm(&Act::Offer(offer)), Some(Confirm::Offer(_))));
    assert!(asks::confirm(&Act::Promise { words: "you'll start".into(), terms: vec![] }).is_none());
    assert!(asks::confirm(&Act::Request(Request::Mentor(3))).is_none());
    let c = asks::confirm(&Act::Request(Request::Position(2))).unwrap();
    assert!(c.words().starts_with("Ask him: move to"));
}

#[test]
fn deals_read_lane_n1s_people() {
    use crate::world::folk::persona::people::People;
    let (mut w, p) = college();
    let i = own(&mut w, p, &[], 0.6);
    let him = Them::Player(w.people[i].player.id);
    let d = Deals { wants: &People };
    // His wants are N1's, and a leaver with nothing on the table says what
    // would keep him, or no.
    assert!(!crate::world::folk::persona::wants::Wanting::wants(&People, &w, him).is_empty());
    assert!(matches!(d.weigh(&w, him, &Offer::default()), Weighed::Counter(_) | Weighed::No));
    let more = Offer { gives: vec![Give::Role(0), Give::Stay, Give::Council], ..Default::default() };
    assert!(deals::lean_after(&w, &d, him, &more).unwrap() < 0.6);
}
