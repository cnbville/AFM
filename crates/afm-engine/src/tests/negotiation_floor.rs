//! The negotiation overhaul, lane NG4 (`docs/design/negotiation-arguments.md`,
//! items 1 and 2): each player has his own lowest number in place of the
//! flat floor, built from his trust in you and the club, your winning and
//! his home, scaled by how much money matters to him; one trust reads your
//! talks and the club's record with him together.

use crate::world::front_office::nfl;
use crate::world::nfl_year::NflTeamSeason;
use crate::world::pro::market::{self, Offer};
use crate::world::pro::{Status, money};
use crate::world::seat::{Job, You};
use crate::world::{World, WorldOptions};

fn nfl_world() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active_in(&w, t, w.year))
        .max_by_key(|&t| w.clubs[t].roster.len())
        .unwrap();
    let teaching = vec![12.0; crate::world::coach::GROUP_COUNT];
    let you: You = serde_json::from_value(serde_json::json!({
        "name": "Test Coach", "age": 45, "ethnicity": "White", "home_state": 0, "ex_pro": false, "reputation": 60.0,
        "teaching": teaching, "scheme": 12.0, "game_management": 12.0,
        "motivation": 12.0, "discipline": 12.0, "recruiting": 12.0, "network": 12.0, "relationships": 12.0,
        "style": { "risk": 10.0, "freedom": 10.0, "physical": 10.0, "ball_hunting": 10.0, "mould": 10.0 },
        "yours": { "play_calling": 12.0, "adaptation": 12.0, "evaluation": 12.0, "negotiating": 12.0 }
    }))
    .unwrap();
    w.start_career(&you, Job::Nfl(t), 7);
    (w, t)
}

/// A mid-priced veteran of yours whose deal just ran out.
fn expiring(w: &mut World, t: usize) -> usize {
    w.today = w.dates.tag_window_open;
    let i = *w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster)
        .find(|&&i| (4.0e6..15.0e6).contains(&nfl::market_read(w, i).0))
        .expect("a mid-priced veteran");
    let year = w.year;
    let c = w.people[i].pro.as_mut().unwrap();
    c.accrued = 5;
    c.contract = Some(money::build(t, money::Kind::Veteran, year - 3, &money::Terms::flat(3, 6.0e6)));
    i
}

/// An offer from club `t` at `apy`, on his ideal's other terms.
fn at(t: usize, ideal: &market::Ideal, apy: f64) -> Offer {
    Offer { team: t, apy, years: ideal.years, gtd: ideal.gtd, start: ideal.start }
}

fn season(w: &mut World, t: usize, wins: u32, losses: u32) {
    let year = w.year - 1;
    w.nfl_seasons.retain(|s| !(s.team == t && s.year == year));
    let s = NflTeamSeason { year, team: t, wins, losses, ties: 0, coach: None, round: 0 };
    w.nfl_seasons.push(s);
}

#[test]
fn the_flat_floor_is_gone_an_offer_under_his_lowest_number_is_short() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t);
    let ideal = market::ideal(&w, i, w.today);
    let m = market::market_apy(&w, i, &ideal);
    let low = market::lowest(&w, i, t, &ideal, true);
    let d = market::discount(&w, i, t, true);
    assert!(low <= m && low >= m * (1.0 - market::FLOOR_MAX) - 1.0, "lowest {low} market {m} {d:?}");
    // At his lowest number the money is met; just under it the offer is
    // short, and scores at most 0.84 whatever the rest (the old floor took
    // 78% of his ideal from anyone whose money goal passed 0.3).
    let under = market::score(&w, i, &at(t, &ideal, low * 0.97), &ideal, Some(t));
    assert!(under <= 0.84 + 1e-9, "under his lowest number: {under}");
    let old_floor = ideal.apy * 0.82;
    if old_floor < low * 0.97 {
        let s = market::score(&w, i, &at(t, &ideal, old_floor), &ideal, Some(t));
        assert!(s <= 0.84 + 1e-9, "the old floor no longer signs him: {s}");
    }
}

#[test]
fn a_man_who_barely_cares_about_money_still_has_a_floor() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t);
    // Love of the game and a rich family: his money goal falls low.
    w.people[i].player.personality.love_of_game = 20.0;
    w.people[i].bg.money = 1.0;
    let g = market::goals(&w, i);
    let ideal = market::ideal(&w, i, w.today);
    let m = market::market_apy(&w, i, &ideal);
    // Before, a money goal of 0.3 or less had no floor: 40% of his market
    // could sign him. Now his discount is capped.
    let s = market::score(&w, i, &at(t, &ideal, m * 0.45), &ideal, Some(t));
    assert!(s <= 0.84 + 1e-9, "money goal {:.2}: 45% of his market scored {s}", g.money);
    let d = market::discount(&w, i, t, true);
    assert!(d.money > 1.0 && d.total <= market::FLOOR_MAX, "{d:?}");
}

#[test]
fn trust_winning_and_home_lower_his_number_and_greed_shrinks_it() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t);
    let you = w.career.as_ref().unwrap().coach;
    // A man who wants to win and trusts you, on a 14-3 club.
    w.people[i].player.personality.competitiveness = 20.0;
    w.people[i].trust_in = you as u32;
    w.people[i].trust = 1.0;
    season(&mut w, t, 14, 3);
    let warm = market::discount(&w, i, t, true);
    assert!(warm.trust > 0.0, "trust counts: {warm:?}");
    if market::goals(&w, i).winning > 0.5 {
        assert!(warm.win > 0.0, "a winner counts: {warm:?}");
    }
    // The same man after a 3-14 year who has stopped trusting you.
    w.people[i].trust = 0.2;
    w.people[i].pro.as_mut().unwrap().trust = 0.4;
    season(&mut w, t, 3, 14);
    let cold = market::discount(&w, i, t, true);
    assert_eq!((cold.trust, cold.win), (0.0, 0.0), "{cold:?}");
    assert!(warm.total > cold.total, "{warm:?} against {cold:?}");
    // A new club gets no trust part.
    let other = (0..w.clubs.len()).find(|&k| k != t && crate::world::franchise::active_in(&w, k, w.year)).unwrap();
    assert_eq!(market::discount(&w, i, other, false).trust, 0.0);
}

#[test]
fn one_trust_reads_your_talks_with_the_clubs_record() {
    let (mut w, t) = nfl_world();
    let i = expiring(&mut w, t);
    let you = w.career.as_ref().unwrap().coach;
    w.people[i].trust_in = you as u32;
    w.people[i].trust = 0.2;
    w.people[i].pro.as_mut().unwrap().trust = 1.0;
    let low = market::one_trust(&w, i, t);
    assert!((low - 0.6).abs() < 1e-6, "the club's full trust and his 0.2 in you: {low}");
    // A good talk raises his trust in you, and with it the contract's.
    let id = w.people[i].player.id;
    crate::world::folk::persona::stores::nudge_trust(&mut w, crate::world::folk::persona::Them::Player(id), 0.6);
    assert!(market::one_trust(&w, i, t) > low + 0.25);
}

/// What the floor does to the AI's own talks before the market, on the
/// test world's rosters: each veteran's re-sign offer (`make_offer` as
/// `resign_talks` makes it) scored with the old flat floor and the new one.
/// A measure, run by hand: `cargo test --release -p afm-engine
/// resign_floor_measure -- --ignored --nocapture`.
#[test]
#[ignore]
fn resign_floor_measure() {
    let (mut w, _) = nfl_world();
    w.today = w.dates.tag_window_open;
    let d = w.today;
    let year = w.year;
    let (mut n, mut old_yes, mut new_yes, mut lost, mut gained) = (0, 0, 0, 0, 0);
    let mut shares: Vec<f64> = Vec::new();
    let mut discounts: Vec<f64> = Vec::new();
    for t in 0..w.clubs.len() {
        if !crate::world::franchise::active_in(&w, t, year) {
            continue;
        }
        for &i in &w.clubs[t].roster.clone() {
            if w.people[i].pro.as_ref().is_none_or(|c| c.status != Status::Roster) {
                continue;
            }
            let (view, _) = market::worth(&w, t, i);
            if market::market_grade(&w, w.people[i].player.position, view) < 68.0 {
                continue;
            }
            let ideal = market::ideal(&w, i, d);
            let Some(o) = market::make_offer(&w, t, i, &ideal, 30, 0.0, year) else { continue };
            let g = market::goals(&w, i);
            let (raw, gshort) = market::raw_score(&w, i, &o, &ideal, Some(t));
            let take = o.apy * (1.0 - market::state_tax(&w, t, year)) / (ideal.apy * 0.95);
            let old = if (take.min(1.2) < 0.82 && g.money > 0.3) || gshort { raw.min(0.84) } else { raw };
            let new = market::score(&w, i, &o, &ideal, Some(t));
            let bar = 0.85 + 0.12 * g.money * (1.0 - g.loyalty);
            n += 1;
            old_yes += (old >= bar) as u32;
            new_yes += (new >= bar) as u32;
            lost += (old >= bar && new < bar) as u32;
            gained += (old < bar && new >= bar) as u32;
            shares.push(o.apy / market::market_apy(&w, i, &ideal));
            discounts.push(market::discount(&w, i, t, true).total);
        }
    }
    shares.sort_by(f64::total_cmp);
    discounts.sort_by(f64::total_cmp);
    let q = |v: &[f64], x: f64| v[((v.len() - 1) as f64 * x) as usize];
    println!(
        "veterans {n}: re-sign yes old {old_yes} new {new_yes} (lost {lost}, gained {gained}); offer/market p10 {:.2} p50 {:.2} p90 {:.2}; discount p50 {:.3} p90 {:.3} max {:.3}",
        q(&shares, 0.1),
        q(&shares, 0.5),
        q(&shares, 0.9),
        q(&discounts, 0.5),
        q(&discounts, 0.9),
        discounts.last().unwrap()
    );
}
