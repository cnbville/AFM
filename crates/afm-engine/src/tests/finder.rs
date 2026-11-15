//! The trade finder (TF1-TF20): the shared offer builder, the values it
//! reads (age curves, the race, the doctors, leverage), your shop over
//! days with its counters, bluffs and closing, a pay-down, and moving a
//! contract nobody wants.

use super::legacy::seat;
use crate::positions::Position;
use crate::world::World;
use crate::world::front_office::{deals, finder};
use crate::world::pro::{self, DeadWhy, bargain, market, trades};

/// A new world's NFL, a few days into the league year. Its deals were all
/// written this year, as if just signed: book them a year back, so the
/// rule against trading a free agent signed this year doesn't stop all.
fn world() -> World {
    let mut w = seat("nfl");
    w.today = w.dates.league_year.date.add(2);
    let year = w.year;
    for p in w.people.iter_mut() {
        if let Some(k) = p.pro.as_mut().and_then(|c| c.contract.as_mut()) {
            k.signed = k.signed.min(year - 1);
        }
    }
    w
}

fn mine(w: &World) -> usize {
    w.seat.and_then(|s| s.nfl_team).unwrap()
}

/// Your healthy players with a deal to trade, by what they're worth to
/// the league's best buyer, highest first.
fn shoppable(w: &World) -> Vec<usize> {
    let t = mine(w);
    let mut v: Vec<(usize, f64)> = w.clubs[t]
        .roster
        .iter()
        .copied()
        .filter(|&i| bargain::movable(w, t, i))
        .map(|i| {
            let best = crate::world::franchise::active_teams_in(w, w.year)
                .into_iter()
                .filter(|&o| o != t)
                .map(|o| trades::player_points(w, o, i))
                .fold(f64::MIN, f64::max);
            (i, best)
        })
        .collect();
    v.sort_by(|a, b| b.1.total_cmp(&a.1));
    v.into_iter().map(|x| x.0).collect()
}

#[test]
fn each_position_fades_on_its_own_curve() {
    // TF14: a back is past it before a quarterback (real aging curves).
    assert!(market::decline(Position::RB, 30) < market::decline(Position::QB, 30));
    assert!(market::decline(Position::RB, 30) < market::decline(Position::WR, 30));
    assert_eq!(market::decline(Position::QB, 32), 1.0);
    // TF15: a clean record costs nothing, a failed one up to 30%.
    assert_eq!(bargain::health_cut(0.2), 0.0);
    assert!(bargain::health_cut(1.4) > bargain::health_cut(0.9));
    assert!(bargain::health_cut(9.0) <= 0.3);
}

#[test]
fn the_builder_pays_with_what_the_buyer_owns_and_leverage_lowers_the_price() {
    let mut w = world();
    let t = mine(&w);
    let list = shoppable(&w);
    let i = list[0];
    let year = w.year;
    let buyer = crate::world::franchise::active_teams_in(&w, year)
        .into_iter()
        .filter(|&o| o != t)
        .max_by(|&a, &b| trades::player_points(&w, a, i).total_cmp(&trades::player_points(&w, b, i)))
        .unwrap();
    let value = trades::player_points(&w, buyer, i).max(200.0);
    let lim = bargain::Limits { back: true, draft_only: None, cond: true, swap: true };
    let pk = bargain::build(&w, buyer, t, i, value, lim).expect("a package");
    assert!(pk.picks.len() <= 5);
    for p in &pk.picks {
        assert!(deals::picks_of(&w, buyer).contains(p), "{p:?} isn't theirs");
    }
    for id in &pk.back {
        let j = crate::world::front_office::index_of(&w, *id).unwrap();
        assert!(bargain::movable(&w, buyer, j));
    }
    assert!(pk.cost <= value * 1.25, "{} for {value}", pk.cost);
    // A trade request takes something off what he'd fetch (TF9).
    let before = bargain::reservation(&w, t, i);
    w.people[i].pro.as_mut().unwrap().wants_out = true;
    let after = bargain::reservation(&w, t, i);
    assert!(before <= 0.0 || after < before, "{before} -> {after}");
    assert!(bargain::leverage(&w, t, i).1.contains(&"He asked to be traded"));
    // The race reads as odds.
    for o in 0..w.nfl.len() {
        let r = bargain::race(&w, o);
        assert!((0.0..=1.0).contains(&r), "{r}");
    }
}

#[test]
fn a_shop_runs_over_days_and_a_deal_closes_through_the_physical() {
    let mut w = world();
    let t = mine(&w);
    let year = w.year;
    let list = shoppable(&w);
    let i = list[0];
    let id = w.people[i].player.id;
    // Clean his record, so the physical doesn't decide the test.
    w.people[i].player.history.clear();
    w.people[i].player.injury_proneness = 4.0;
    w.people[i].player.wear = 0.0;
    finder::start(&mut w, id, true, finder::Ask::Best, vec![], 0.5, false).unwrap();
    let s = w.front_office.trades.shop.clone().unwrap();
    assert!(!s.bids.is_empty());
    assert!(s.bids.iter().all(|b| b.state != finder::State::Offer), "no club answers the same day");
    // An open shop is news, and he heard it (TF19).
    assert!(s.leaked || s.open);
    // Two days on, the answers are in, each offer with its expiry (TF1).
    for _ in 0..2 {
        w.today = w.today.add(1);
        let d = w.today;
        finder::day(&mut w, d);
    }
    let s = w.front_office.trades.shop.clone().unwrap();
    assert!(s.bids.iter().all(|b| b.state != finder::State::Waiting));
    let offers: Vec<&finder::Bid> = s.bids.iter().filter(|b| b.state == finder::State::Offer).collect();
    assert!(!offers.is_empty(), "no club made an offer: {:?}", s.bids.iter().map(|b| &b.state).collect::<Vec<_>>());
    for b in &offers {
        assert!(b.until >= w.today);
        // He opens below his aim, at or under his top (TF7).
        assert!(b.bid <= b.aim + 1e-6 && b.aim <= b.top + 1e-6, "{} {} {}", b.bid, b.aim, b.top);
    }
    // A bluff he catches costs trust for a year: his margin with you rises.
    let club = offers[0].club;
    let rival = crate::world::franchise::active_teams_in(&w, year)
        .into_iter()
        .filter(|&o| o != t && o != club)
        .min_by(|&a, &b| finder::believable(&w, club, a, i).total_cmp(&finder::believable(&w, club, b, i)))
        .unwrap();
    let before = deals::margin(&w, club, t);
    let mut caught = false;
    for _ in 0..6 {
        if w.front_office
            .trades
            .shop
            .as_ref()
            .unwrap()
            .bids
            .iter()
            .any(|b| b.club == club && b.state != finder::State::Offer)
        {
            break;
        }
        let _ = finder::rival(&mut w, club, rival, true);
        if w.front_office.trades.distrust.iter().any(|x| x.0 == club) {
            caught = true;
            break;
        }
        w.today = w.today.add(1);
    }
    if caught {
        assert!(deals::margin(&w, club, t) > before);
    }
    // Accept the best offer still standing: it closes through the physical.
    w.front_office.trades.distrust.clear();
    let s = w.front_office.trades.shop.clone().unwrap();
    let Some(best) = s.bids.iter().filter(|b| b.state == finder::State::Offer).max_by(|a, b| a.bid.total_cmp(&b.bid))
    else {
        return;
    };
    let buyer = best.club;
    let pd = finder::paydown_amount(&w, i, 0.5);
    finder::accept(&mut w, buyer).unwrap();
    for _ in 0..3 {
        w.today = w.today.add(1);
        let d = w.today;
        finder::day(&mut w, d);
    }
    let s = w.front_office.trades.shop.clone().unwrap();
    let c = s.closing.clone().unwrap();
    if c.revised.is_some() {
        finder::revised(&mut w, true).unwrap();
    }
    let s = w.front_office.trades.shop.clone().unwrap();
    let c = s.closing.unwrap();
    match c.result.as_deref() {
        Some("Done") => {
            assert_eq!(w.people[i].pro.as_ref().unwrap().team, Some(buyer));
            assert!(s.closed);
            // The pay-down stays on your cap as dead money (TF12).
            if pd > 0.0 {
                let dead: f64 = w.clubs[t]
                    .dead
                    .iter()
                    .filter(|d| d.player == id && d.year == year && d.why == DeadWhy::PaidDown)
                    .map(|d| d.amount)
                    .sum();
                assert!((dead - pd).abs() < 1.0, "{dead} vs {pd}");
            }
            for p in &c.package.picks {
                assert!(deals::owns(&w, t, p), "{p:?} didn't come to you");
            }
        }
        Some(why) => assert!(!why.is_empty()),
        None => panic!("the closing never finished: {c:?}"),
    }
}

#[test]
fn a_contract_nobody_wants_has_a_price_club_by_club() {
    let mut w = world();
    let t = mine(&w);
    // Your worst deal: the player worth least to you.
    let i = w.clubs[t]
        .roster
        .iter()
        .copied()
        .filter(|&i| bargain::movable(&w, t, i))
        .min_by(|&a, &b| trades::player_points(&w, t, a).total_cmp(&trades::player_points(&w, t, b)))
        .unwrap();
    let id = w.people[i].player.id;
    let (_, list) = finder::dumps(&w, id).unwrap();
    assert!(!list.is_empty(), "no club takes him on any terms");
    assert!(list.windows(2).all(|x| x[0].cost <= x[1].cost), "not cheapest first");
    for d in &list {
        assert!(d.dead >= 0.0);
        for p in &d.picks {
            assert!(deals::owns(&w, t, p));
        }
    }
    let club = list[0].club;
    finder::dump(&mut w, id, club).unwrap();
    assert_eq!(w.people[i].pro.as_ref().unwrap().team, Some(club));
    let _ = pro::space(&w, t, w.year);
}
