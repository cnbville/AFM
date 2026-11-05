//! Lane NB9, the NIL update's first items: two pots and NIL Go (B18), and a
//! player's first answer to your money (N1).

use super::legacy::seat;
use crate::calendar::Date;
use crate::world::front_office::college::{
    MoneyKind, Mood, budget, collective_deals, reaction, set_deal, set_promise, usual_deals,
};
use crate::world::recruiting as rec;
use crate::world::{Stage, World, ncaa};

/// A college seat with money in both pots and a few players on the roster
/// (a world with no history has no college players yet, so young pros go
/// back to college). The seat's world sits early in 2025, before the House
/// settlement; `house` moves it to August, with two pots.
fn college(house: bool) -> (World, usize) {
    let mut w = seat("college");
    assert!(!ncaa::two_pots(w.today), "{:?}", w.today);
    if house {
        w.today = Date::ymd(2025, 8, 1);
    }
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
    (w, j)
}

#[test]
fn nil_go_reviews_by_the_rules_of_each_date() {
    let d = Date::ymd;
    assert!(!ncaa::two_pots(d(2025, 6, 30)) && ncaa::two_pots(d(2025, 7, 1)));
    assert!(!ncaa::nil_go_reviews(d(2025, 6, 10), 50_000.0, 50_000.0), "before NIL Go opened");
    assert!(ncaa::nil_go_reviews(d(2025, 7, 1), 600.0, 600.0));
    assert!(!ncaa::nil_go_reviews(d(2025, 7, 1), 599.0, 599.0), "under $600");
    // April 2026: up to $2,500 skips while his year stays at $15,000.
    assert!(!ncaa::nil_go_reviews(d(2026, 5, 1), 2_000.0, 10_000.0));
    assert!(ncaa::nil_go_reviews(d(2026, 5, 1), 2_000.0, 20_000.0));
    assert!(ncaa::nil_go_reviews(d(2026, 5, 1), 3_000.0, 3_000.0));
    // July 2026: $600-15,000 only once his deals pass $50,000.
    assert!(!ncaa::nil_go_reviews(d(2026, 8, 1), 10_000.0, 40_000.0));
    assert!(ncaa::nil_go_reviews(d(2026, 8, 1), 10_000.0, 60_000.0));
    assert!(ncaa::nil_go_reviews(d(2026, 8, 1), 20_000.0, 20_000.0));
    // A reviewed deal is cleared to the top of his range.
    assert_eq!(rec::nil_go(d(2026, 8, 1), 1.0e6, 200_000.0), 200_000.0);
    assert_eq!(rec::nil_go(d(2025, 6, 1), 1.0e6, 200_000.0), 1.0e6);
    assert_eq!(rec::nil_go(d(2026, 8, 1), 10_000.0, 1_000.0), 10_000.0, "under the review line");
}

#[test]
fn the_school_pays_its_share_and_nil_go_reads_the_collectives() {
    let (w, j) = college(true);
    let prog = &w.programs[j];
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool).unwrap();
    let top = rec::nil_go_top_recruit(&w.people[i]);
    assert!(top > 0.0);
    let money = 4.0 * top;
    // Before NIL Go opened (June 11, 2025) it is all the collective's, and
    // nothing reviews it.
    let (s, c) = rec::pots(prog, Date::ymd(2025, 6, 1), money, top);
    assert_eq!((s, c), (0.0, money));
    // After, the school's 12 of 20 million is its own; the collective's part
    // is cleared to his top.
    let (s, c) = rec::pots(prog, w.today, money, top);
    assert!((s - 0.6 * money).abs() < 1.0, "{s}");
    assert!((c - top).abs() < 1.0, "{c} against {top}");
}

#[test]
fn your_money_is_the_schools_revenue_share() {
    let (mut w, j) = college(true);
    let b = budget(&w, j);
    assert!(b.two_pots);
    assert!((b.yours - 12.0e6).abs() < 1.0, "the school's pot is yours: {}", b.yours);
    assert!((b.collective - 8.0e6).abs() < 1.0);
    assert!(b.collective_roster > 0.0 && b.collective_roster <= 0.6 * 8.0e6 + 1.0, "{}", b.collective_roster);
    let own = collective_deals(&w, j);
    assert_eq!(own.len(), usual_deals(&w, j).len(), "a collective deal for every player");
    let (id, usual) = usual_deals(&w, j)[0];
    assert!(set_deal(&mut w, id, 12.5e6).is_err(), "more than the school's pot");
    set_deal(&mut w, id, 1.0e6).unwrap();
    // With NIL at your staff his usual share is what the deal replaces.
    let replaced =
        if w.front_office.owner(crate::world::front_office::Duty::Nil) == crate::world::front_office::Owner::You {
            0.0
        } else {
            usual
        };
    let left = budget(&w, j).left;
    assert!((left - (b.left - 1.0e6 + replaced)).abs() < 1.0, "a deal comes out of the revenue share: {left}");
}

#[test]
fn a_player_answers_your_deal_at_once_and_nothing_moves_yet() {
    for house in [false, true] {
        deal_answers(house);
    }
}

fn deal_answers(house: bool) {
    let (mut w, j) = college(house);
    let (id, usual) = usual_deals(&w, j).into_iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    assert!(usual > 25_000.0, "{usual}");
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    let lean = w.people[i].cal.lean;
    let text = set_deal(&mut w, id, 0.0).unwrap();
    let r = reaction(&w, id, MoneyKind::Deal).expect("an answer");
    assert!(text.contains(&r.text), "{text}");
    assert!(matches!(r.mood, Mood::WantsMore | Mood::Insulted), "{:?}: {}", r.mood, r.text);
    set_deal(&mut w, id, 3.0 * usual).unwrap();
    let r = reaction(&w, id, MoneyKind::Deal).unwrap();
    assert_eq!(r.mood, Mood::Thrilled, "{}", r.text);
    assert_eq!(w.front_office.college.reactions.iter().filter(|r| r.id == id).count(), 1, "the latest only");
    assert_eq!(w.people[i].cal.lean, lean, "the answer only reads: July 1 does the rest");
}

#[test]
fn a_recruit_answers_your_promise_at_once_and_decides_in_his_own_time() {
    for house in [false, true] {
        promise_answers(house);
    }
}

fn promise_answers(house: bool) {
    let (mut w, j) = college(house);
    let class = rec::next_class(&w);
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool && p.class_of == class).unwrap();
    let id = w.people[i].player.id;
    let usual = rec::usual_nil_offer(&w, j, i);
    assert!(usual > 1_000.0, "{usual}");
    let commit = w.people[i].cal.commit;
    let text = set_promise(&mut w, id, Some(10.0 * usual + 1.0e6)).unwrap();
    let r = reaction(&w, id, MoneyKind::Promise).expect("an answer");
    assert!(text.contains(&r.text), "{text}");
    assert!(matches!(r.mood, Mood::Thrilled | Mood::Pleased), "{:?}: {}", r.mood, r.text);
    assert!(r.text.contains("only a number"), "no scholarship offer yet: {}", r.text);
    set_promise(&mut w, id, Some(0.0)).unwrap();
    let r = reaction(&w, id, MoneyKind::Promise).unwrap();
    assert!(matches!(r.mood, Mood::WantsMore | Mood::Insulted), "{:?}: {}", r.mood, r.text);
    assert_eq!(w.people[i].cal.commit.map(|c| c.0), commit.map(|c| c.0), "his decision keeps its own clock");
}
