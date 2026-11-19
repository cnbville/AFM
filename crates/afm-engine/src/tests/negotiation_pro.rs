//! The negotiation overhaul, lane NG5 (`docs/design/negotiation-arguments.md`,
//! items 3a, 3b and 3c at the NFL): a true point about your pro's season
//! talks his agent's ask down toward his market, never below it; once one
//! has landed his agent counters with a prove-it deal on the stat you
//! named; a proud man you made the point to wants to see the market.

use super::persona_negotiate::{expiring, nfl_world, offer};
use crate::world::World;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::memory::Point;
use crate::world::folk::persona::negotiate::{Facet, Market, contract, points};
use crate::world::folk::persona::terms::{Give, Valuing, Weighed};
use crate::world::front_office::nfl;
use crate::world::pro::{ProSeason, market, money};

/// A point kept for him this season on a stat at his spot, at `pull`.
fn point(w: &mut World, i: usize, pull: f32) -> &'static str {
    let pos = w.people[i].player.position;
    let s = points::STATS.iter().find(|s| s.spots.contains(&pos)).expect("a stat at his spot");
    let id = w.people[i].player.id;
    let today = w.today;
    w.folk.live.persona.points.push(Point { with: Them::Player(id), day: today, key: s.key.into(), pull });
    s.key
}

/// A full season on his record, so a prove-it clause has a number to read.
fn a_season(w: &mut World, i: usize) {
    let year = w.year - 1;
    let c = w.people[i].pro.as_mut().unwrap();
    c.seasons.push(ProSeason { year, games: 16, starts: 16, ..Default::default() });
}

#[test]
fn a_true_point_talks_his_ask_down_to_his_market_never_below() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    let id = w.people[i].player.id;
    let kind = nfl::TalkKind::ReSign;
    let before = nfl::ideal_argued(&w, i, kind, 0.0);
    let m = market::market_apy(&w, i, &before);
    let ask0 = nfl::asking(&w, id).unwrap();
    // Half the way in the same line, then all of it kept for the season.
    let half = nfl::ideal_argued(&w, i, kind, 0.5);
    point(&mut w, i, 1.0);
    let all = nfl::ideal_argued(&w, i, kind, 0.0);
    assert!(half.apy <= before.apy && all.apy <= half.apy, "{} {} {}", before.apy, half.apy, all.apy);
    // Never below the ideal his market meets, and his market itself stays.
    assert!(all.apy >= m / market::MARKET_SLACK - 1.0, "{} under {}", all.apy, m / market::MARKET_SLACK);
    assert!((market::market_apy(&w, i, &all) - m).abs() < 1.0);
    let ask1 = nfl::asking(&w, id).unwrap();
    assert!(ask1.apy <= ask0.apy, "his agent's ask comes down: {} {}", ask0.apy, ask1.apy);
    // The lowest number reads the same market with or without the points.
    let own = market::lowest(&w, i, t, &all, true);
    assert!((own - market::lowest(&w, i, t, &before, true)).abs() < 1.0);
}

#[test]
fn once_a_point_lands_his_agent_counters_prove_it_on_that_stat() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    a_season(&mut w, i);
    let id = w.people[i].player.id;
    let credited = w.people[i].pro.as_ref().unwrap().credited;
    let low = offer(vec![Give::Money(money::minimum(w.year, credited) * 2.0), Give::Years(3), Give::Guaranteed(0.3)]);
    let p = contract::proposal(&w, i, &low).unwrap();
    // Before any point: a plain counter, no clause.
    let j0 = nfl::judge(&w, id, &p).unwrap();
    assert!(j0.prove_it.is_none());
    point(&mut w, i, 0.4);
    let j = nfl::judge(&w, id, &p).unwrap();
    let ask = j.ask.expect("his ask");
    let u = j.prove_it.expect("a prove-it clause");
    assert!(ask.years <= 2, "a short deal: {ask:?}");
    assert!(ask.incentives > 0.0 && ask.apy < j0.ask.unwrap().apy, "the gap on incentives: {ask:?}");
    let ideal = nfl::ideal_argued(&w, i, nfl::TalkKind::ReSign, 0.0);
    assert!(ask.gtd <= ideal.gtd + 0.03 + 1e-9, "no more guarantee than he wanted: {ask:?}");
    // The talk's counter carries the clause.
    let f = Facet::Agent { client: i, agent: crate::world::folk::persona::negotiate::agent_of(&w, i) };
    let market = Market { facet: f, base: &crate::world::folk::persona::talk::DEALS };
    let him = Them::Agent(crate::world::folk::persona::negotiate::agent_of(&w, i).unwrap());
    match market.weigh(&w, him, &low) {
        Weighed::Counter(c) => assert_eq!(c.terms, vec![u]),
        Weighed::Lowball => {}
        v => panic!("expected a counter, got {v:?}"),
    }
}

#[test]
fn a_proud_man_wants_to_see_the_market_after_a_point() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    let kind = nfl::TalkKind::ReSign;
    w.people[i].player.personality.traits.ego = 20;
    let bar0 = nfl::own_bar(&w, i, kind);
    point(&mut w, i, 0.3);
    let bar1 = nfl::own_bar(&w, i, kind);
    assert!(bar1 > bar0, "{bar0} {bar1}");
    // An average man's bar doesn't move.
    let j = expiring_other(&mut w, t, i);
    w.people[j].player.personality.traits.ego = 10;
    let b0 = nfl::own_bar(&w, j, kind);
    point(&mut w, j, 0.3);
    assert!((nfl::own_bar(&w, j, kind) - b0).abs() < 1e-12);
}

/// A second expiring veteran of yours.
fn expiring_other(w: &mut World, t: usize, skip: usize) -> usize {
    let year = w.year;
    let j = *w.clubs[t]
        .roster
        .iter()
        .find(|&&j| j != skip && w.people[j].pro.as_ref().is_some_and(|c| c.contract.is_some()))
        .unwrap();
    let c = w.people[j].pro.as_mut().unwrap();
    c.accrued = 5;
    c.contract = Some(money::build(t, money::Kind::Veteran, year - 3, &money::Terms::flat(3, 6.0e6)));
    j
}

/// Lane NG6 (item 4): "take less and I'll sign you help" lowers a winner's
/// number as he believes it; broken, his trust in the club falls and his
/// next deal with you costs more.
#[test]
fn a_help_promise_lowers_a_winners_number_and_breaking_it_costs_his_next_deal() {
    use crate::world::folk::persona::terms::HELP_ANY;
    use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t, true);
    let id = w.people[i].player.id;
    w.people[i].player.personality.competitiveness = 20.0;
    assert!(market::goals(&w, i).winning > 0.5);
    let part = nfl::help_part(&w, i, HELP_ANY);
    assert!(part > 0.0 && part <= nfl::HELP_DISCOUNT, "{part}");
    // His market as his ideal reads it (the tiny world's other clubs may
    // not bid, which leaves the agent's read at the minimum).
    let ideal = market::ideal(&w, i, w.today);
    let plain = market::lowest_with(&w, i, t, &ideal, true, 0.0);
    assert!(market::lowest_with(&w, i, t, &ideal, true, part) < plain);
    // A man who doesn't care about winning gets nothing from it.
    w.people[i].player.personality.competitiveness = 0.0;
    if market::goals(&w, i).winning <= 0.5 {
        assert_eq!(nfl::help_part(&w, i, HELP_ANY), 0.0);
    }
    w.people[i].player.personality.competitiveness = 20.0;
    // Said and never kept: by the opener no new starter came.
    talks::promise(&mut w, PromiseTo::Player(id), PromiseKind::Help(HELP_ANY), 0);
    let due = w.folk.promises.last().unwrap().due;
    assert!(due > w.dates.nfl_kickoff.add(-365) && due.days_to(w.dates.nfl_kickoff.add(368)) >= 0);
    let trust = w.people[i].pro.as_ref().unwrap().trust;
    w.today = due;
    talks::check_promises(&mut w, due);
    let c = w.people[i].pro.as_ref().unwrap();
    assert_eq!(w.folk.promises.last().unwrap().kept, Some(false));
    assert!(c.trust < trust, "his trust in the club falls");
    let (_, premium) = c.help_owed.expect("his next deal costs more");
    assert!(premium > 0.0);
    let with = nfl::ideal_argued(&w, i, nfl::TalkKind::ReSign, 0.0).apy;
    w.people[i].pro.as_mut().unwrap().help_owed = None;
    let without = nfl::ideal_argued(&w, i, nfl::TalkKind::ReSign, 0.0).apy;
    assert!((with / without - 1.0 - premium as f64).abs() < 1e-6, "{with} {without} {premium}");
}
