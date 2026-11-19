//! Multi-year NIL deals (the new NIL scheme, item 12, lane B): terms,
//! what a guarantee is worth to a player, the bound on later budgets,
//! July paying each year, buyouts, and your desk's terms.

use crate::calendar::Date;
use crate::player::PlayingTime;
use crate::world::front_office::college::{contract_of, deal_of, fill_deals, set_deal, set_term, usual_deals};
use crate::world::nil_bid::{self, DealKind, NilDeal};
use crate::world::{Stage, World, ncaa};

fn deal(year: i32, through: i32) -> NilDeal {
    NilDeal { year, amount: 1.0e6, kind: DealKind::Signed, through }
}

#[test]
fn a_deal_pays_each_year_of_its_term_then_ends() {
    let d = deal(2027, 2029);
    assert!(d.contract() && d.pays(2027) && d.pays(2029) && !d.pays(2030) && !d.pays(2026));
    assert_eq!(d.years_left(), 3);
    // Half of what is left: the Big 12 template.
    assert_eq!(d.buyout(), 1.5e6);
    let d = d.after_payday(2027).expect("two years left");
    assert_eq!((d.year, d.years_left(), d.buyout()), (2028, 2, 1.0e6));
    let d = d.after_payday(2028).and_then(|d| d.after_payday(2029));
    assert!(d.is_none(), "done after its last July");
    // A one-year deal owes nothing to leave and ends after its July.
    let one = deal(2027, 0);
    assert!(!one.contract() && one.buyout() == 0.0 && one.after_payday(2027).is_none());
    assert_eq!(one.after_payday(2026), Some(one), "a deal for next July waits");
}

#[test]
fn a_term_goes_to_blue_chips_and_top_transfers_from_two_pots() {
    let house = Date::ymd(2026, 1, 10);
    let before = Date::ymd(2025, 1, 10);
    // A five-star who will play, a four-star who starts early; not a
    // three-star, and nothing before revenue sharing.
    assert_eq!(nil_bid::recruit_term(house, 0.8), nil_bid::TERM_RECRUIT);
    assert_eq!(nil_bid::recruit_term(house, 0.25), nil_bid::TERM_RECRUIT);
    assert_eq!(nil_bid::recruit_term(house, 0.07), 1);
    assert_eq!(nil_bid::recruit_term(before, 0.8), 1);
    // A transfer: a starter well over the level's starters with two
    // seasons or more left, up to three years.
    assert_eq!(nil_bid::transfer_term(house, PlayingTime::Starter, 5.0, 2), 2);
    assert_eq!(nil_bid::transfer_term(house, PlayingTime::Starter, 5.0, 4), 3);
    assert_eq!(nil_bid::transfer_term(house, PlayingTime::Starter, 1.0, 3), 1);
    assert_eq!(nil_bid::transfer_term(house, PlayingTime::Rotation, 9.0, 3), 1);
    assert_eq!(nil_bid::transfer_term(house, PlayingTime::Starter, 9.0, 1), 1);
}

#[test]
fn he_weighs_the_guarantee_by_his_makeup() {
    let mut w = super::legacy::seat("college");
    w.add_class_aged(17);
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool).expect("a recruit");
    let p = &mut w.people[i];
    // A family that needs the money and a loyal kid deciding for himself.
    p.bg.money = 0.05;
    p.bg.safety = 0.2;
    p.family.decider = crate::world::background::Decider::Parent;
    p.player.personality.traits.loyalty = 16;
    p.player.personality.traits.ambition = 8;
    p.player.personality.traits.ego = 8;
    let secure = nil_bid::guarantee(&w.people[i]);
    assert!(secure > 0.1 && secure <= nil_bid::GUARANTEE_TOP, "{secure}");
    assert_eq!(nil_bid::term_taken(secure, 3), 3);
    assert!((nil_bid::term_weight(secure, 3) - (1.0 + 2.0 * secure)).abs() < 1e-9);
    // A rich, ambitious, vain kid wants to sell himself again.
    let p = &mut w.people[i];
    p.bg.money = 0.95;
    p.bg.safety = 0.95;
    p.family.decider = crate::world::background::Decider::Himself;
    p.player.personality.traits.loyalty = 5;
    p.player.personality.traits.ambition = 18;
    p.player.personality.traits.ego = 17;
    let free = nil_bid::guarantee(&w.people[i]);
    assert!(free < 0.0, "{free}");
    assert_eq!(nil_bid::term_taken(free, 3), 1, "he signs for one year");
    assert_eq!(nil_bid::term_weight(free, 3), 1.0, "and a term adds nothing for him");
}

#[test]
fn later_budgets_are_bound() {
    let budget = 30.0e6;
    let bar = nil_bid::TERM_BOUND * budget;
    // Plenty of room: the whole term.
    assert_eq!(nil_bid::bound_term(budget, &[0.0; 4], 1.0e6, 3), 3);
    // Next year's room is full: no term at all.
    assert_eq!(nil_bid::bound_term(budget, &[0.0, bar, 0.0, 0.0], 1.0e6, 3), 1);
    // The third year is full: two years.
    assert_eq!(nil_bid::bound_term(budget, &[0.0, 0.0, bar, 0.0], 1.0e6, 3), 2);
}

/// A college seat with money and a roster in two pots (as `nil.rs` builds
/// it).
fn college() -> (World, usize) {
    let mut w = super::legacy::seat("college");
    w.today = Date::ymd(2025, 8, 1);
    for age in [15, 16, 17] {
        w.add_class_aged(age);
    }
    let j = crate::world::front_office::program(&w).expect("a college seat");
    for p in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(30) {
        p.stage = Stage::College;
        p.pro = None;
        p.college = Some(j);
        p.college_seasons = 1;
    }
    w.rebuild_rosters();
    let c = &mut w.programs[j].collective;
    c.budget = 20.0e6;
    c.school = 12.0e6;
    w.nil_book = crate::world::nil_bid::NilBook::from_pots(&w);
    (w, j)
}

#[test]
fn your_multi_year_deal_can_be_raised_not_lowered() {
    let (mut w, j) = college();
    assert!(ncaa::two_pots(w.today));
    // The term's rules, not the pot: no budget limit.
    w.set_sandbox_switch(crate::world::sandbox::Switch::NilBudget, true);
    fill_deals(&mut w);
    let (id, _) = usual_deals(&w, j).into_iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    // A player who values a guarantee.
    let p = &mut w.people[i];
    p.bg.money = 0.05;
    p.bg.safety = 0.2;
    p.player.personality.traits.loyalty = 17;
    p.player.personality.traits.ambition = 6;
    p.player.personality.traits.ego = 6;
    p.college_seasons = 1;
    p.class_of = 2024;
    set_deal(&mut w, id, Some(0.0), Some(0.0)).unwrap();
    let first = set_term(&mut w, id, 2).unwrap_err();
    assert!(first.contains("Set his deal first"), "{first}");
    set_deal(&mut w, id, Some(200_000.0), Some(0.0)).unwrap();
    let text = set_term(&mut w, id, 2).unwrap();
    assert!(text.contains("guaranteed for 2 years"), "{text}");
    let (through, amount) = contract_of(&w, id).expect("a contract");
    assert_eq!(amount, 200_000.0);
    assert!(through > w.today.year(), "{through}");
    // Lowering is refused; a raise raises the guarantee too.
    assert!(set_deal(&mut w, id, Some(100_000.0), None).is_err());
    set_deal(&mut w, id, Some(250_000.0), None).unwrap();
    assert_eq!(contract_of(&w, id).unwrap().1, 250_000.0);
    assert_eq!(deal_of(&w, id).unwrap().0, 250_000.0);
    // He would owe half of what is left if he left.
    let d = w.people[i].cal.nil_deal.unwrap();
    assert_eq!(d.buyout(), 0.5 * 250_000.0 * d.years_left() as f64);
    // A man who wants to sell himself again says no.
    let (other, _) = usual_deals(&w, j).into_iter().find(|d| d.0 != id).unwrap();
    let k = w.people.iter().position(|p| p.player.id == other).unwrap();
    let p = &mut w.people[k];
    p.bg.money = 0.95;
    p.bg.safety = 0.95;
    p.player.personality.traits.loyalty = 4;
    p.player.personality.traits.ambition = 19;
    p.player.personality.traits.ego = 18;
    p.class_of = 2024;
    set_deal(&mut w, other, Some(100_000.0), Some(0.0)).unwrap();
    let no = set_term(&mut w, other, 3).unwrap();
    assert!(no.contains("one year at a time"), "{no}");
    assert!(contract_of(&w, other).is_none());
}
