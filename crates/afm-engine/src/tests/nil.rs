//! Lane NB9, the NIL update's first items: two pots and NIL Go (B18), and a
//! player's first answer to your money (N1).

use super::legacy::seat;
use crate::calendar::Date;
use crate::world::front_office::college::{
    MoneyKind, Mood, budget, clear_deals, collective_deals, deal_of, fill_deals, offer, race, reaction, set_deal,
    set_promise, set_usual_deals, stage_board, usual_deals, weighed_offer,
};
use crate::world::front_office::{Duty, Owner};
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
    // NIL Go's book, as July 2025's payday would have read it.
    if house {
        w.nil_book = crate::world::nil_bid::NilBook::from_pots(&w);
        // The room AI schools paid their arrivals last July (a class and
        // transfers), which your roster split keeps for yours to come.
        w.nil_book.new_share = [0.3; 4];
    }
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
    let top = rec::go_top_recruit(&w, j, &w.people[i]);
    assert!(top > 0.0);
    let money = 4.0 * top;
    // Before NIL Go opened (June 11, 2025) it is all the collective's, and
    // nothing reviews it.
    let (s, c) = rec::pots(prog, Date::ymd(2025, 6, 1), money, top, false);
    assert_eq!((s, c), (0.0, money));
    // After, the school's 12 of 20 million is its own; the collective's part
    // is cleared to his top, and what NIL Go denies moves into the school's
    // revenue share as far as its room under the cap goes (the new NIL
    // scheme, item 11).
    let (s, c) = rec::pots(prog, w.today, money, top, false);
    let room = ncaa::share_room(prog, crate::world::nil_bid::deal_year(w.today));
    let moved = (0.4 * money - top).min(room).max(0.0);
    assert!((s - 0.6 * money - moved).abs() < 1.0, "{s}");
    assert!((c - top).abs() < 1.0, "{c} against {top}");
}

#[test]
fn your_money_is_the_schools_revenue_share() {
    use crate::player::{Level, OptF64, PlayingTime};
    let (mut w, j) = college(true);
    // The collective pays the men with a college season (a man with none
    // signed for the scholarship), so the seat's players get one, and the
    // book reads its bars on them.
    for i in w.programs[j].roster.clone() {
        let p = &mut w.people[i];
        p.last_role = Some(PlayingTime::Starter);
        p.last_performance = OptF64::some(60.0);
        p.player.history = vec![season_row(Level::FBS, PlayingTime::Starter, 60.0)];
    }
    w.nil_book = crate::world::nil_bid::NilBook::from_pots(&w);
    w.nil_book.new_share = [0.3; 4];
    let (id, usual) = usual_deals(&w, j)[0];
    // Every player of yours starts on the going rate (Cain 2026-11-07:
    // every NIL dollar is yours, so nobody is left to a split).
    fill_deals(&mut w);
    let b = budget(&w, j);
    assert!(b.two_pots);
    assert!((b.yours - 12.0e6).abs() < 1.0, "the school's pot is yours: {}", b.yours);
    assert!((b.collective - 8.0e6).abs() < 1.0);
    // The going rate is an AI school's split, keeping room for a class and
    // transfers to come (0.3 of the money here), each deal rounded to the
    // nearest $500.
    let deals_n = collective_deals(&w, j).len() as f64;
    assert!(
        b.collective_roster > 0.0 && b.collective_roster <= 0.7 * 8.0e6 + 500.0 * deals_n,
        "{}",
        b.collective_roster
    );
    let own = collective_deals(&w, j);
    assert_eq!(own.len(), usual_deals(&w, j).len(), "a collective deal for every player");
    let (had, _) = deal_of(&w, id).expect("a deal for every player");
    assert!((had - usual).abs() <= 250.0, "the going rate to $500: {had} against {usual}");
    assert!(set_deal(&mut w, id, Some(12.5e6), None).is_err(), "more than the school's pot");
    set_deal(&mut w, id, Some(1.0e6), None).unwrap();
    let left = budget(&w, j).left;
    assert!((left - (b.left - 1.0e6 + had)).abs() < 1.0, "a deal comes out of the revenue share: {left}");
}

/// NIL is always yours (Cain 2026-11-07): there is no staff to hand it to,
/// your deals stand from one money year to the next, a $0 box pays nothing
/// until you fill it, and the collective deal is yours to set too.
#[test]
fn every_nil_dollar_is_yours_and_your_deals_stand() {
    use crate::world::front_office::{Duty, Owner};
    let (mut w, j) = college(true);
    w.front_office.set_owner(Duty::Nil, Owner::Staff);
    assert_eq!(w.front_office.owner(Duty::Nil), Owner::You);
    fill_deals(&mut w);
    let (id, usual) = usual_deals(&w, j).into_iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    // A collective deal of yours, cleared by NIL Go.
    let b = budget(&w, j);
    let (_, old_c) = deal_of(&w, id).unwrap();
    set_deal(&mut w, id, None, Some(50_000.0)).unwrap();
    assert_eq!(deal_of(&w, id).unwrap().1, 50_000.0);
    let moved = b.coll_left - budget(&w, j).coll_left;
    assert!((moved - (50_000.0 - old_c)).abs() < 1.0, "the collective's pot pays it: {moved}");
    assert!(set_deal(&mut w, id, None, Some(1.0e9)).is_err(), "more than the collective has");
    // A new money year keeps every deal.
    let before = w.front_office.college.deals.clone();
    w.today = Date::ymd(2026, 8, 1);
    fill_deals(&mut w);
    assert_eq!(deal_of(&w, id).unwrap().1, 50_000.0, "your deal stands");
    for d in &before {
        assert!(w.front_office.college.deals.contains(d), "{d:?}");
    }
    // Clear every deal: each box is $0 and pays nothing until filled.
    let n = clear_deals(&mut w).unwrap();
    assert!(n > 0);
    assert_eq!(deal_of(&w, id), Some((0.0, 0.0)));
    fill_deals(&mut w);
    assert_eq!(deal_of(&w, id), Some((0.0, 0.0)), "a $0 box is a deal of nothing, not a blank");
    // Fill blanks puts the going rate back in every empty box.
    let (filled, blank) = set_usual_deals(&mut w).unwrap();
    assert!(filled > 0 && filled <= blank, "{filled} of {blank}");
    let now = usual_deals(&w, j).into_iter().find(|d| d.0 == id).unwrap().1;
    assert!((deal_of(&w, id).unwrap().0 - now).abs() <= 250.0 && usual > 0.0);
}

/// From the review of Cain's 2026-11-07 change: a recruit who signed
/// before you took over starts on the old staff's money, a screen's read
/// writes nothing, a player who leaves loses his deals, and a desk from
/// before is brought up once on load.
#[test]
fn inherited_signees_old_desks_and_screens_keep_their_money() {
    use crate::world::front_office::college::{NIL_REV, after_load, deals_now, deals_year};
    let (mut w, j) = college(true);
    // He signed with the program before your desk existed.
    let class = rec::next_class(&w);
    // The senior the old staff would have paid the least of those it pays
    // at all, so its usual money is more than nothing (a boy no other
    // school offered is priced at next to nothing, the first senior in the
    // list can be one, and one outside its July paid set gets none) and
    // his waiting deal leaves the class's room as it was.
    let usual_of = |w: &World, i: usize| {
        let (s, c) = crate::world::front_office::college::usual_parts(w, j, w.people[i].player.id);
        s + c
    };
    let k = (0..w.people.len())
        .filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].class_of == class)
        .filter(|&i| usual_of(&w, i) > 500.0)
        .min_by(|&a, &b| usual_of(&w, a).total_cmp(&usual_of(&w, b)))
        .unwrap();
    assert!(usual_of(&w, k) > 500.0, "a senior worth the old staff's money: {}", usual_of(&w, k));
    let signee = w.people[k].player.id;
    w.people[k].cal.signed = Some(crate::world::Signed {
        program: j,
        walk_on: false,
        date: w.today,
        early_enrollee: false,
        flipped: false,
        committed: None,
    });
    fill_deals(&mut w);
    let had = w.front_office.college.signed.iter().find(|x| x.0 == signee).copied();
    assert!(had.is_some_and(|x| x.1 + x.2 > 0.0), "the old staff's usual money: {had:?}");
    assert_eq!(w.front_office.college.nil_rev, NIL_REV, "a new desk is on the new rules");
    // The roster you inherit keeps the old staff's deals.
    let (id, usual) = usual_deals(&w, j).into_iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    assert!(
        usual > 0.0 && (deal_of(&w, id).unwrap().0 - usual).abs() <= 250.0,
        "{:?} against {usual}",
        deal_of(&w, id)
    );
    // A player without a deal has none: nothing is filled in for you, on a
    // screen or on July 1 (Cain 2026-11-08), and a read writes nothing.
    w.front_office.college.deals.retain(|d| d.0 != id);
    w.front_office.college.coll_deals.retain(|d| d.0 != id);
    let before = serde_json::to_string(&w.front_office).unwrap();
    let (s, c) = deals_now(&w, j).unwrap();
    assert!(!s.contains_key(&id) && !c.contains_key(&id), "no going rate is filled in");
    assert!(budget(&w, j).roster > 0.0);
    assert_eq!(serde_json::to_string(&w.front_office).unwrap(), before, "a read never writes");
    fill_deals(&mut w);
    assert!(deal_of(&w, id).is_none(), "nor on the day: {:?}", deal_of(&w, id));
    // A desk from before: the deals you set stand at their numbers, whatever
    // money year they were for, and nobody gets the going rate.
    w.front_office.college.nil_rev = 0;
    w.front_office.college.deals_year = deals_year(&w) - 1;
    for d in w.front_office.college.deals.iter_mut() {
        d.1 = 1.0;
    }
    let set = w.front_office.college.deals.len();
    let said = |w: &World, what: &str| w.calendar.on(w.today).iter().filter(|e| e.text.contains(what)).count();
    after_load(&mut w);
    assert_eq!(w.front_office.college.nil_rev, NIL_REV);
    assert_eq!(w.front_office.college.deals.len(), set, "nobody is filled in");
    assert!(w.front_office.college.deals.iter().all(|d| d.1 == 1.0), "your deals stand");
    assert!(deal_of(&w, id).is_none());
    assert_eq!(said(&w, "no deal yet"), 1, "the inbox says who has no deal");
    after_load(&mut w);
    // A desk the last version brought up keeps its deals and hears why.
    w.front_office.college.nil_rev = 1;
    after_load(&mut w);
    assert_eq!(w.front_office.college.deals.len(), set);
    assert_eq!(said(&w, "The last version put the going rate"), 1);
    // A player who leaves loses his deals.
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    let x = (0..w.programs.len()).find(|&x| x != j && w.programs[x].division == w.programs[j].division).unwrap();
    w.people[i].college = Some(x);
    w.rebuild_rosters();
    fill_deals(&mut w);
    assert!(deal_of(&w, id).is_none(), "{:?}", deal_of(&w, id));
}

/// Cain's answers (2026-11-08): the game never names money for you. A man
/// who joins later has no deal until you set one, and a recruit your
/// coordinator offered signs for nothing, his offer having named none.
#[test]
fn nothing_is_filled_in_for_you() {
    use crate::world::front_office::college::after_stage;
    let (mut w, j) = college(true);
    fill_deals(&mut w);
    let i = w.people.iter().position(|p| p.stage == Stage::Pro).unwrap();
    let id = w.people[i].player.id;
    let p = &mut w.people[i];
    p.stage = Stage::College;
    p.pro = None;
    p.college = Some(j);
    p.college_seasons = 1;
    w.rebuild_rosters();
    fill_deals(&mut w);
    assert!(deal_of(&w, id).is_none(), "{:?}", deal_of(&w, id));
    assert!(budget(&w, j).roster > 0.0, "the roster you inherited keeps its deals");
    let class = rec::next_class(&w);
    let k = w
        .people
        .iter()
        .position(|p| {
            p.stage == Stage::HighSchool
                && p.class_of == class
                && !w.front_office.college.board.iter().any(|t| t.id == p.player.id)
        })
        .unwrap();
    let rid = w.people[k].player.id;
    w.people[k].cal.signed = Some(crate::world::Signed {
        program: j,
        walk_on: false,
        date: w.today,
        early_enrollee: false,
        flipped: false,
        committed: None,
    });
    after_stage(&mut w, j, &[rid], false);
    let t = w.front_office.college.board.iter().find(|t| t.id == rid).expect("on your board as his");
    assert!(t.nil.is_none() && t.nil_coll.is_none());
    assert_eq!(w.front_office.college.signed.iter().find(|x| x.0 == rid).map(|x| (x.1, x.2)), Some((0.0, 0.0)));
}

/// Stands the clock at your next section's first minute without playing
/// anything (the answer only waits for its hour).
fn to_next_section(w: &mut World) {
    let (d, m) = w.next_section_stop();
    w.today = d;
    w.played = m + 1;
}

#[test]
fn a_player_answers_your_deal_at_your_next_section_and_nothing_moves_yet() {
    for house in [false, true] {
        deal_answers(house);
    }
}

fn deal_answers(house: bool) {
    let (mut w, j) = college(house);
    // His answers, not the pot: no budget limit.
    w.set_sandbox_switch(crate::world::sandbox::Switch::NilBudget, true);
    let (id, usual) = usual_deals(&w, j).into_iter().max_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
    assert!(usual > 25_000.0, "{usual}");
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    let lean = w.people[i].cal.lean;
    // The six-part day: he answers at your next section.
    let text = set_deal(&mut w, id, Some(0.0), None).unwrap();
    assert!(text.contains("He'll get back to you by"), "{text}");
    assert!(reaction(&w, id, MoneyKind::Deal).is_none(), "no answer before its hour");
    let now = (w.today, w.played);
    to_next_section(&mut w);
    let r = reaction(&w, id, MoneyKind::Deal).expect("an answer");
    assert!(matches!(r.mood, Mood::WantsMore | Mood::Insulted), "{:?}: {}", r.mood, r.text);
    (w.today, w.played) = now;
    set_deal(&mut w, id, Some(3.0 * usual), None).unwrap();
    to_next_section(&mut w);
    let r = reaction(&w, id, MoneyKind::Deal).unwrap();
    assert_eq!(r.mood, Mood::Thrilled, "{}", r.text);
    assert_eq!(w.front_office.college.reactions.iter().filter(|r| r.id == id).count(), 1, "the latest only");
    assert_eq!(w.people[i].cal.lean, lean, "the answer only reads: July 1 does the rest");
}

#[test]
fn a_recruit_answers_your_promise_at_your_next_section_and_decides_in_his_own_time() {
    for house in [false, true] {
        promise_answers(house);
    }
}

fn promise_answers(house: bool) {
    let (mut w, j) = college(house);
    let class = rec::next_class(&w);
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool && p.class_of == class).unwrap();
    // A four-star, whom the market pays real money (the new NIL scheme
    // prices a low-star kid at a few hundred dollars).
    w.people[i].stars = 4;
    let id = w.people[i].player.id;
    let usual = rec::usual_nil_offer(&w, j, i);
    assert!(usual > 1_000.0, "{usual}");
    let commit = w.people[i].cal.commit;
    // Far over his usual money, inside the pot it comes from (his place in
    // your July paid set floors it, Cain 2026-11-19: a four-star on this
    // thin roster of men with no college season takes a large part of the
    // school's share by his standing, so the whole budget is no bound).
    let promise = (10.0 * usual + 1.0e6).min(0.9 * budget(&w, j).left);
    let text = set_promise(&mut w, id, Some(promise), None).unwrap();
    assert!(text.contains("He'll get back to you by"), "{text}");
    assert!(reaction(&w, id, MoneyKind::Promise).is_none(), "no answer before its hour");
    let now = (w.today, w.played);
    to_next_section(&mut w);
    let r = reaction(&w, id, MoneyKind::Promise).expect("an answer");
    assert!(matches!(r.mood, Mood::Thrilled | Mood::Pleased), "{:?}: {}", r.mood, r.text);
    assert!(r.text.contains("only a number"), "no scholarship offer yet: {}", r.text);
    (w.today, w.played) = now;
    // Another school offers him, with money of its own: nothing from you
    // is under it now.
    let x = (0..w.programs.len()).find(|&x| x != j && w.programs[x].division == w.programs[j].division).unwrap();
    w.programs[x].collective.budget = 20.0e6;
    w.programs[x].collective.school = 12.0e6;
    w.people[i].cal.offers.push(x);
    set_promise(&mut w, id, Some(0.0), None).unwrap();
    to_next_section(&mut w);
    let r = reaction(&w, id, MoneyKind::Promise).unwrap();
    assert!(matches!(r.mood, Mood::WantsMore | Mood::Insulted), "{:?}: {}", r.mood, r.text);
    assert_eq!(w.people[i].cal.commit.map(|c| c.0), commit.map(|c| c.0), "his decision keeps its own clock");
}

// --- NIL pull (playtest item F3, Cain 2026-11-12) -------------------------------------------

/// Money that tops every other offer counts once more by how many times it
/// tops them, up to three, by his own money weight, and the hungrier the
/// steeper.
#[test]
fn the_clear_top_offer_counts_once_more_by_its_ratio() {
    let w = rec::top_money_weight(0.13, 1.0);
    assert!((w - 0.6 * 0.13).abs() < 1e-12, "{w}");
    // Money that tops no one gets nothing.
    assert_eq!(rec::top_money(w, 100_000.0, 100_000.0), 0.0);
    assert_eq!(rec::top_money(w, 80_000.0, 100_000.0), 0.0);
    // Double is double at $40K or at $400K.
    let (small, big) = (rec::top_money(w, 80_000.0, 40_000.0), rec::top_money(w, 800_000.0, 400_000.0));
    assert!((small - big).abs() < 1e-12 && (small - w * 2f64.ln()).abs() < 1e-12, "{small} {big}");
    // It stops growing at three times the next offer.
    assert_eq!(rec::top_money(w, 1.0e6, 100_000.0), rec::top_money(w, 300_000.0, 100_000.0));
    assert!(rec::top_money(w, 250_000.0, 100_000.0) < rec::top_money(w, 300_000.0, 100_000.0));
    // A multiple of pocket money is no multiple: the next offer reads as $10,000 at least.
    assert_eq!(rec::top_money(w, 9_000.0, 500.0), 0.0);
    assert!((rec::top_money(w, 20_000.0, 500.0) - w * 2f64.ln()).abs() < 1e-12);
    // The hungrier, the steeper; a transfer's heavier weight carries over.
    assert!(rec::top_money_weight(0.26, 1.0) > 2.5 * w);
    assert!(rec::top_money_weight(0.065, 1.0) < 0.4 * w);
    assert!((rec::top_money_weight(0.13, 2.0) - 2.0 * w).abs() < 1e-12);
}

/// A transfer weighs money more than the recruit he was, by his situation.
#[test]
fn a_transfer_weighs_money_by_his_situation() {
    let more = rec::transfer_more;
    // Three seasons left at FBS and no pro future: the usual figure.
    assert!((more(3, Some(0.0), true) - 1.75).abs() < 1e-12);
    // His last season ahead and no pro future: the most.
    assert!(more(1, Some(-3.0), true) > more(2, Some(-3.0), true));
    assert!(more(2, Some(-3.0), true) > more(3, Some(-3.0), true));
    // A real pro prospect weighs it less, whatever he has left.
    assert!(more(1, Some(7.0), true) < more(3, Some(0.0), true));
    assert!(more(3, Some(7.0), true) < more(3, Some(3.0), true));
    // Coming from below FBS, more (Cain: "especially when coming from < FBS").
    assert!(more(2, Some(-3.0), false) > more(2, Some(-3.0), true));
    // Always more than a recruit, never past three times.
    for left in 0..6 {
        for over in [-10.0, 0.0, 3.0, 6.0, 12.0] {
            for fbs in [true, false] {
                let m = more(left, Some(over), fbs);
                assert!((1.2..=3.0).contains(&m), "{left} {over} {fbs}: {m}");
            }
        }
    }
}

/// In a recruit's race your money is read against the best other money on
/// his list, and theirs against yours.
#[test]
fn double_the_money_moves_a_recruit_more_than_the_dollars_alone() {
    let (mut w, j) = college(true);
    // Room for double the best offer, which his place in a $20M school's
    // July paid set floors (Cain 2026-11-19).
    w.programs[j].collective.budget = 100.0e6;
    w.programs[j].collective.school = 60.0e6;
    let (i, id) = lone_recruit(&mut w, None);
    let others: Vec<usize> =
        (0..w.programs.len()).filter(|&x| x != j && w.programs[x].division == w.programs[j].division).take(2).collect();
    // Each prices against the other's top bid (the new NIL scheme): the
    // $20M school pays half its top, the $5M school its whole top, half
    // as much.
    for (&x, budget) in others.iter().zip([20.0e6, 5.0e6]) {
        w.programs[x].collective.budget = budget;
        w.programs[x].collective.school = 0.6 * budget;
        w.people[i].cal.offers.push(x);
    }
    offer(&mut w, id).unwrap();
    let money: Vec<f64> = others.iter().map(|&x| rec::usual_nil_offer(&w, x, i)).collect();
    let (best, next) = (money[0].max(money[1]), money[0].min(money[1]));
    assert!(next > 10_000.0 && best > next, "{money:?}");
    let list = [j, others[0], others[1]];
    let at = |m: f64| rec::appeals(&w, i, &list, Some((j, m)));
    let weight = rec::reason_weights(&w, i)[2];
    let lift = rec::top_money_weight(weight, 1.0);
    assert!(lift > 0.0);
    let dollars = |m: f64| weight * 1.5 * (m / 1.0e6).powf(0.6);
    let (none, same, double, six) = (at(0.0), at(best), at(2.0 * best), at(6.0 * best));
    // Matching the best money earns nothing extra; double earns ln 2 of his
    // lift on top of the dollars; six times earns no more than three.
    assert!((same[0] - none[0] - dollars(best)).abs() < 1e-9);
    assert!((double[0] - same[0] - (dollars(2.0 * best) - dollars(best)) - lift * 2f64.ln()).abs() < 1e-9);
    assert!((six[0] - same[0] - (dollars(6.0 * best) - dollars(best)) - lift * 3f64.ln()).abs() < 1e-9);
    // The school that paid the most led the money until you topped it.
    let rich = if money[0] > money[1] { 1 } else { 2 };
    assert!((none[rich] - double[rich] - lift * (best / next).min(3.0).ln()).abs() < 1e-9);
    assert!((none[3 - rich] - double[3 - rich]).abs() < 1e-12, "the third school never led the money");
    // His reasons still add up to the appeal, with the lift under money.
    set_promise(&mut w, id, Some(2.0 * best), None).unwrap();
    let parts = rec::reason_parts(&w, i, j, None);
    assert!((parts.iter().map(|x| x.0).sum::<f64>() - rec::appeal(&w, i, j, None)).abs() < 1e-9);
    assert!((parts[2].0 - dollars(2.0 * best) - lift * 2f64.ln()).abs() < 1e-9, "{}", parts[2].0);
    assert!((rec::appeal(&w, i, j, None) - double[0]).abs() < 1e-9);
}

// --- Cain's NIL bug report (2026-11-06) ----------------------------------------------------

/// A four-star of the class signing next with no offers yet, as Cain's
/// guard was.
fn lone_recruit(w: &mut World, skip: Option<u32>) -> (usize, u32) {
    let class = rec::next_class(w);
    let i = w
        .people
        .iter()
        .position(|p| p.stage == Stage::HighSchool && p.class_of == class && Some(p.player.id) != skip)
        .unwrap();
    let p = &mut w.people[i];
    p.stars = 4;
    p.cal.offers.clear();
    p.cal.early_offers.clear();
    p.cal.commit = None;
    (i, p.player.id)
}

#[test]
fn an_offer_needs_no_money_and_a_lone_one_leads_and_is_never_a_lowball() {
    for house in [false, true] {
        lone_offer(house);
    }
}

fn lone_offer(house: bool) {
    let (mut w, j) = college(house);
    let (i, id) = lone_recruit(&mut w, None);
    let (school, collective) = rec::usual_nil_parts(&w, j, i);
    assert!(school + collective > 1_000.0, "a four-star's usual money: {school} + {collective}");
    offer(&mut w, id).unwrap();
    // No number: the scholarship alone (every NIL dollar is yours, Cain
    // 2026-11-07: a blank box is no money).
    assert!(weighed_offer(&w, j, id).abs() < 1.0, "{}", weighed_offer(&w, j, id));
    let b = stage_board(&w).unwrap();
    let o = b.offers.iter().find(|o| o.person == i).expect("the offer goes out");
    assert!(o.nil.is_some_and(|x| x.abs() < 1.0), "{:?}", o.nil);
    // His only offer leads his list, whatever the money.
    assert_eq!(race(&w, id).first().map(|r| r.0), Some(j));
    // With nobody else paying, nothing from you is no lowball.
    set_promise(&mut w, id, Some(0.0), None).unwrap();
    to_next_section(&mut w);
    let r = reaction(&w, id, MoneyKind::Promise).expect("an answer");
    assert_eq!(r.mood, Mood::Fine, "{}", r.text);
    assert!(r.text.contains("No other school"), "{}", r.text);
    // A number is what he weighs.
    set_promise(&mut w, id, Some(50_000.0), None).unwrap();
    assert!((weighed_offer(&w, j, id) - 50_000.0).abs() < 1.0);
    to_next_section(&mut w);
    assert_eq!(reaction(&w, id, MoneyKind::Promise).unwrap().mood, Mood::Pleased);
    // In two pots your collective deal adds what NIL Go clears of it; before,
    // there is one number.
    if house {
        set_promise(&mut w, id, Some(50_000.0), Some(collective.max(10_000.0))).unwrap();
        let top = rec::go_top_recruit(&w, j, &w.people[i]);
        let c = rec::nil_go(w.today, collective.max(10_000.0), top);
        assert!((weighed_offer(&w, j, id) - 50_000.0 - c).abs() < 1.0, "{} against {c}", weighed_offer(&w, j, id));
    } else {
        assert!(set_promise(&mut w, id, Some(50_000.0), Some(10_000.0)).is_err(), "one pot before July 2025");
    }
    // With your coordinator on the board his offers name no money either:
    // only you do (Cain 2026-11-08, "I want the only offering control").
    set_promise(&mut w, id, None, None).unwrap();
    w.front_office.set_owner(Duty::Board, Owner::Staff);
    assert!(weighed_offer(&w, j, id).abs() < 1.0, "{}", weighed_offer(&w, j, id));
    let o = stage_board(&w).unwrap().offers.into_iter().find(|o| o.person == i).unwrap();
    assert!(o.nil.is_some_and(|x| x.abs() < 1.0), "{:?}", o.nil);
}

#[test]
fn your_offer_before_his_class_first_wave_is_an_early_one() {
    let (mut w, j) = college(false);
    let class = rec::next_class(&w);
    assert!(w.today < rec::first_wave(class), "{:?} {class}", w.today);
    // The class signing next, before its first wave: his junior spring.
    let (i, id) = lone_recruit(&mut w, None);
    offer(&mut w, id).unwrap();
    assert_eq!(w.people[i].cal.early_offers.iter().map(|o| o.0).collect::<Vec<_>>(), vec![j]);
    // From the first wave on an offer is no early one.
    w.today = rec::first_wave(class).add(1);
    assert_eq!(rec::next_class(&w), class);
    let (k, other) = lone_recruit(&mut w, Some(id));
    offer(&mut w, other).unwrap();
    assert!(w.people[k].cal.early_offers.is_empty(), "{:?}", w.people[k].cal.early_offers);
    assert_eq!(w.people[k].cal.offers, vec![j]);
}

// --- High school NIL (N3) ---------------------------------------------------------------

/// A world with a class of 17-year-olds early in 2026 and no state open yet
/// (the clock opens them), plus a five-star quarterback starter, first-team
/// all-state, moved to a school in `code`.
fn hs_star(code: &str) -> (World, usize) {
    let mut w = World::empty(crate::world::WorldOptions { scale: 0.004, ..Default::default() });
    w.year = 2025;
    w.today = Date::ymd(2026, 2, 4);
    w.add_class_aged(17);
    let s = crate::world::geo::state_index(code).expect("a state");
    let school = w.high_schools.iter().position(|h| h.state == s).expect("a school there");
    let i = w.people.iter().position(|p| p.stage == Stage::HighSchool).expect("a high schooler");
    let p = &mut w.people[i];
    p.hs = school;
    p.stars = 5;
    p.player.position = crate::positions::Position::QB;
    p.last_role = Some(crate::player::PlayingTime::Starter);
    p.cal.all_state = 2;
    p.player.personality.traits.aura = 14;
    p.born = Date::ymd(2008, 9, 1);
    (w, i)
}

/// Opens every state whose rule is open to all by 2025, as the clock does.
fn open_states(w: &mut World) {
    for st in w.states.iter_mut() {
        let rule = crate::world::data::hs_nil_rule(&st.code);
        st.hs_nil = rule.scope == crate::world::data::HsNilScope::All && rule.from.is_some_and(|y| y <= 2025);
    }
}

#[test]
fn the_state_rules_come_from_the_data() {
    use crate::world::data::{HsNilScope as S, hs_nil_rule};
    assert_eq!(hs_nil_rule("CA").scope, S::All);
    assert_eq!(hs_nil_rule("CA").from, Some(2021));
    for code in ["AL", "HI", "MS", "SC", "WY"] {
        assert_eq!(hs_nil_rule(code).scope, S::Barred, "{code}");
        assert_eq!(hs_nil_rule(code).from, None, "{code}");
    }
    assert_eq!(hs_nil_rule("AR").scope, S::SignedInState);
    assert_eq!(hs_nil_rule("MO").scope, S::SignedInState);
    assert_eq!(hs_nil_rule("TX").scope, S::CollegeAfterHs);
    assert_eq!(hs_nil_rule("TX").min_age, 17);
    // Research corrections (N3): Kansas since 2021's ruling, Wisconsin since
    // its 2025 vote, Ohio from its November 2025 referendum's next season.
    assert_eq!(hs_nil_rule("KS").from, Some(2022));
    assert_eq!(hs_nil_rule("WI").from, Some(2025));
    assert_eq!(hs_nil_rule("OH").from, Some(2026));
    assert!(!hs_nil_rule("FL").note.is_empty());
}

#[test]
fn a_high_schooler_earns_only_where_his_state_lets_him() {
    use crate::world::fame;
    // Open in California: a five-star quarterback is offered six figures.
    let (mut w, i) = hs_star("CA");
    assert!(!fame::hs_nil_open(&w, &w.people[i]), "no state open before the clock opens it");
    assert_eq!(fame::hs_nil_money(&w, i, fame::fame(&w, i)), 0.0);
    open_states(&mut w);
    assert!(fame::hs_nil_open(&w, &w.people[i]));
    let f = fame::fame(&w, i);
    let offered = fame::hs_nil_offered(&w.people[i], f);
    assert!(offered > 10_000.0, "fame {f:.3}, offered {offered:.0}");
    let money = fame::hs_nil_money(&w, i, f);
    assert!(money == offered || money == 0.0, "his family takes it or turns it down");
    // A three-star backup in the same school is offered nothing.
    let p = &mut w.people[i];
    p.stars = 3;
    p.last_role = Some(crate::player::PlayingTime::Backup);
    p.cal.all_state = 0;
    assert_eq!(fame::hs_nil_offered(&w.people[i], fame::fame(&w, i)), 0.0);
    // Barred in Alabama and, while he plays, in Texas.
    for code in ["AL", "TX"] {
        let (mut w, i) = hs_star(code);
        open_states(&mut w);
        assert!(!fame::hs_nil_open(&w, &w.people[i]), "{code}");
        assert_eq!(fame::hs_nil_money(&w, i, fame::fame(&w, i)), 0.0, "{code}");
    }
    // Missouri: only once he signs with a Missouri college.
    let (mut w, i) = hs_star("MO");
    open_states(&mut w);
    assert!(!fame::hs_nil_open(&w, &w.people[i]));
    let mo = crate::world::geo::state_index("MO").unwrap();
    let ks = crate::world::geo::state_index("KS").unwrap();
    let signed = |w: &mut World, s: usize| {
        let j = w.programs.iter().position(|c| c.state == s).expect("a college there");
        w.people[i].cal.signed = Some(crate::world::Signed {
            program: j,
            walk_on: false,
            date: w.today,
            early_enrollee: false,
            flipped: false,
            committed: None,
        });
    };
    signed(&mut w, ks);
    assert!(!fame::hs_nil_open(&w, &w.people[i]), "signed out of state");
    signed(&mut w, mo);
    assert!(fame::hs_nil_open(&w, &w.people[i]), "signed at home");
}

#[test]
fn the_year_book_pays_high_schoolers_in_open_states_only() {
    let (mut w, _) = hs_star("CA");
    open_states(&mut w);
    crate::world::fame::year_book(&mut w);
    let hs: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).collect();
    for &i in &hs {
        let p = &w.people[i];
        if p.brand > 0.0 {
            assert!(crate::world::fame::hs_nil_open(&w, p), "{} earns in a closed state", p.name);
            assert!(p.brand >= 200.0, "no deal under the smallest: {}", p.brand);
        }
    }
    // Under a few in a hundred earn anything (real: about 1% or less).
    let earning = hs.iter().filter(|&&i| w.people[i].brand > 0.0).count();
    assert!(earning * 20 < hs.len(), "{earning} of {}", hs.len());
}

#[test]
fn an_old_world_file_takes_the_limited_states_off_and_fills_the_deals() {
    use crate::world::fame;
    let (mut w, i) = hs_star("CA");
    open_states(&mut w);
    // The old data had Arkansas open to all from 2021 and Missouri from 2023.
    let ar = crate::world::geo::state_index("AR").unwrap();
    let mo = crate::world::geo::state_index("MO").unwrap();
    w.states[ar].hs_nil = true;
    w.states[mo].hs_nil = true;
    // Missouri opened by a later state vote stays open.
    w.changes.applied.push(crate::calendar::rules::AppliedChange {
        change: crate::calendar::Change::HsNil,
        season: 2027,
        detail: "MO".to_string(),
    });
    fame::hs_nil_after_load(&mut w);
    assert!(!w.states[ar].hs_nil && w.states[mo].hs_nil);
    let expected = fame::hs_nil_money(&w, i, fame::fame(&w, i)) as f32;
    assert_eq!(w.people[i].brand, expected, "his deals as February's book sets them");
    // A newer save is left exactly as it was.
    w.people[i].brand = 1234.0;
    fame::hs_nil_after_load(&mut w);
    assert_eq!(w.people[i].brand, 1234.0);
}

/// Minor bug 335: once he has played college ball the market prices a
/// player on his play more than his recruiting stars, and the interior
/// line on both sides earns less than the tackles and edge rushers.
#[test]
fn a_college_players_pay_reads_his_play_and_the_interior_line_costs_less() {
    use crate::player::PlayingTime::Starter;
    use crate::positions::Position as P;
    let w = |pos, stars, edge| rec::nil_weight(Some(Starter), pos, stars, edge);
    // Stars fade: a two-star starter is 0.6 of a four-star at the same play
    // (the square root before gave 0.28); a freshman keeps the full read.
    let r = w(P::WR, 2, 0.0) / w(P::WR, 4, 0.0);
    assert!((r - 0.08f64.powf(0.2)).abs() < 1e-9, "{r}");
    let fresh = rec::nil_weight(None, P::WR, 2, 0.0) / rec::nil_weight(None, P::WR, 4, 0.0);
    assert!((fresh - 0.08).abs() < 1e-9, "{fresh}");
    // Ten points of play over the level's starters: 3% a point.
    let up = w(P::CB, 3, 10.0) / w(P::CB, 3, 0.0);
    assert!((up - 0.3f64.exp()).abs() < 1e-9, "{up}");
    // The interior line: guards and centers 0.58 of an offensive tackle,
    // defensive tackles 0.73 of an edge rusher.
    assert!((w(P::IOL, 3, 0.0) / w(P::OT, 3, 0.0) - 0.58).abs() < 1e-9);
    assert!((w(P::DT, 3, 0.0) / w(P::EDGE, 3, 0.0) - 0.73).abs() < 1e-9);
}

/// Minor bug 336 and the NIL split by value (2026-11-24): in two pots the
/// school spreads its whole revenue share over the roster by each man's
/// own standing and his position. The third party's part is its own test
/// (`the_third_party_fills_each_man_to_his_market_price`).
#[test]
fn the_school_spreads_its_whole_share_by_each_mans_value() {
    use crate::positions::Position as P;
    use crate::world::nil_bid::star_bid;
    // $20M a year, 60% the school's: a player holding 1/50 of the school's
    // weights and 1/20 of the collective's.
    let (school, _) = rec::july_split(20.0e6, 0.6, (1.0, 50.0), (1.0, 20.0), true);
    assert!((school - 12.0e6 / 50.0).abs() < 1.0, "{school}");
    // Before two pots it is all the collective's, which keeps some back.
    let (school, coll) = rec::july_split(20.0e6, 0.0, (1.0, 50.0), (1.0, 20.0), false);
    assert_eq!(school, 0.0);
    assert!((coll - 20.0e6 * rec::NIL_ROSTER / 20.0).abs() < 1.0, "{coll}");
    // Quarterbacks first, then the lines, then receivers; specialists last.
    let at = rec::share_at;
    assert!(at(P::QB) > at(P::DT) && at(P::DT) > at(P::OT));
    assert!(at(P::OT) > at(P::WR) && at(P::WR) > at(P::TE));
    assert!(at(P::TE) > at(P::K));
    // A Power 4 school's median paid man (a four-star's read) and anyone
    // over him is paid his position's whole worth.
    let w = |pos, standing| rec::share_weight(pos, standing, 0, true);
    let (mid, low) = (star_bid(4), star_bid(3));
    for standing in [mid, 0.6, 1.0, 1.5] {
        assert!((w(P::DT, standing) / w(P::WR, standing) - at(P::DT)).abs() < 1e-9, "{standing}");
        assert!((w(P::TE, standing) / w(P::WR, standing) - at(P::TE)).abs() < 1e-9, "{standing}");
    }
    // Under him it fades: a three-star's read is paid the same at every
    // position, and nothing reads below that.
    assert!((w(P::QB, low) - low).abs() < 1e-12 && (w(P::K, low) - low).abs() < 1e-12);
    assert!((w(P::QB, 0.01) - 0.01).abs() < 1e-12);
    let between = w(P::QB, (mid * low).sqrt()) / w(P::WR, (mid * low).sqrt());
    assert!(between > 1.0 && between < at(P::QB), "{between}");
    // Over him the worth of the school's first quarterback keeps growing:
    // one quarterback plays. Its other quarterbacks are paid as any spot.
    assert!((w(P::QB, mid) / w(P::WR, mid) - at(P::QB)).abs() < 1e-9);
    assert!(w(P::QB, 1.0) / w(P::WR, 1.0) > 1.5 * at(P::QB));
    let second = |standing| rec::share_weight(P::QB, standing, 0, false);
    assert!((second(1.0) / w(P::WR, 1.0) - at(P::QB)).abs() < 1e-9);
    assert_eq!(second(low), w(P::QB, low));
    assert_eq!(rec::share_weight(P::OT, 1.0, 0, false), w(P::OT, 1.0), "only a quarterback's grows");
    // More standing is more money at every position, and none is none.
    for pos in [P::QB, P::OT, P::WR, P::TE, P::K] {
        let v: Vec<f64> = [0.0, 0.01, low, 0.15, mid, 0.5, 1.0, 1.4].iter().map(|&x| w(pos, x)).collect();
        assert!(v[0] == 0.0 && v.windows(2).all(|x| x[0] < x[1]), "{pos:?} {v:?}");
    }
    // The other levels pay 29% of a roster, so their median paid man is a
    // median starter.
    let g5 = |pos, standing| rec::share_weight(pos, standing, 1, true);
    assert!((g5(P::DT, 1.0) / g5(P::WR, 1.0) - at(P::DT)).abs() < 1e-9);
    assert!((g5(P::DT, mid) - g5(P::WR, mid)).abs() < 1e-12, "a four-star's read is far under him there");
}

/// The NIL split by value (2026-11-24): a man's standing in his school's
/// revenue share is his own. A starter reads his play against the level's
/// median starter at his spot, anyone else with a season the share of
/// those starters he outplays, with the year he grows, and a man with no
/// play to read his stars.
#[test]
fn a_mans_standing_in_the_share_is_his_own() {
    use crate::player::{Level, OptF64, PlayingTime as R};
    use crate::positions::Position as P;
    use crate::world::nil_bid::{BID_PLAY, ShareBars, star_bid, star_bid_at};
    let (mut w, j) = college(true);
    // Ten starters at each spot at every level, from 50 to 68.
    let mut bars = ShareBars::default();
    for l in bars.play.iter_mut() {
        for b in l.iter_mut() {
            *b = (0..10).map(|k| 50.0 + 2.0 * k as f32).collect();
        }
    }
    bars.lift = [0.0, 0.5, 1.0, 1.0];
    assert_eq!(bars.median(0, P::WR), Some(60.0));
    // The k-th of ten sits at (k + 0.5) / 10, linear between two of them.
    assert_eq!(bars.outplays(0, P::WR, 49.0), 0.0);
    assert!((bars.outplays(0, P::WR, 50.0) - 0.05).abs() < 1e-12);
    assert!((bars.outplays(0, P::WR, 61.0) - 0.6).abs() < 1e-12);
    assert!((bars.outplays(0, P::WR, 68.0) - 0.95).abs() < 1e-12);
    assert_eq!(bars.outplays(0, P::WR, 68.5), 1.0);
    let season = w.today.year();
    let i = w.programs[j].roster[0];
    let standing = |w: &World| rec::share_standing(&w.people[i], 0, season, &bars);
    {
        let p = &mut w.people[i];
        p.player.position = P::WR;
        p.stars = 3;
        p.cal.nil_role = None;
        p.roads.away_years = 0;
        // No college season on his record: his stars, at his school's level.
        let row = crate::player::SeasonRecord {
            position: P::WR,
            age: 18,
            level: Level::HS,
            playing_time: R::Starter,
            overall_start: 60.0,
            overall: 60.0,
            potential: 60.0,
            performance: OptF64::some(70.0),
            level_rating: None,
            level_rating_end: None,
            coaching: 10.0,
            events: Default::default(),
            games_missed: 0,
            adjustment: OptF64::NONE,
            spring_gain: OptF64::NONE,
            spring_open: OptF64::NONE,
            team: crate::player::OptIdx::NONE,
            depth: crate::player::DepthLabel::NONE,
            situation: OptF64::NONE,
            results: OptF64::NONE,
        };
        p.player.history = vec![row];
        p.last_role = Some(R::Starter);
        p.last_performance = OptF64::some(70.0);
    }
    assert!(!rec::college_season(&w.people[i]));
    assert!((standing(&w) - star_bid(3)).abs() < 1e-12, "a newcomer's high school season reads nothing");
    assert!((rec::share_standing(&w.people[i], 1, season, &bars) - star_bid_at(3.5)).abs() < 1e-12);
    // A starter: 3% a point over the median starter, with the market's floor.
    w.people[i].player.history.last_mut().unwrap().level = Level::FBS;
    assert!((standing(&w) - (BID_PLAY * 10.0).exp()).abs() < 1e-12);
    w.people[i].last_performance = OptF64::some(20.0);
    assert!((standing(&w) - (BID_PLAY * -15.0).exp()).abs() < 1e-12);
    // A backup going into his third year or later: the share of the
    // starters he outplays, half a point ahead of last season.
    w.people[i].last_role = Some(R::Backup);
    w.people[i].last_performance = OptF64::some(60.5);
    w.people[i].class_of = season - 2;
    let old = standing(&w);
    assert!((old - 1.5 * 0.6).abs() < 1e-9, "{old}");
    // Going into his second year he is read 4.5 points ahead.
    w.people[i].class_of = season - 1;
    let young = standing(&w);
    assert!((young - 1.5 * 0.8).abs() < 1e-9, "{young}");
    // A man under every starter is worth nothing to the share yet, one
    // over all of them more than a median starter.
    w.people[i].last_performance = OptF64::some(40.0);
    assert_eq!(standing(&w), 0.0);
    w.people[i].last_performance = OptF64::some(80.0);
    assert!((standing(&w) - 1.5).abs() < 1e-12);
    // A transfer is read in the place he landed.
    w.people[i].cal.nil_role = Some((R::Starter, 0.0));
    w.people[i].last_performance = OptF64::some(64.0);
    assert!((standing(&w) - (BID_PLAY * 4.0).exp()).abs() < 1e-12);
    // A season he sat out reads his stars again.
    w.people[i].last_performance = OptF64::NONE;
    assert!((standing(&w) - star_bid(3)).abs() < 1e-12);
}

/// Cain 2026-11-17 ("go"): the roster you inherit at a new job keeps each
/// player's market price, not the payroll split. A star paid $4.2M last
/// July keeps at least that, a deal signed for the coming year stands as
/// written, the rest stay on the split, the old staff's deals are marked
/// until you set your own, and a star feels paid, not cheated.
#[test]
fn the_roster_you_inherit_keeps_its_market_price() {
    use crate::world::front_office::college::{deals_year, market_prices, ready};
    use crate::world::nil_bid::{DealKind, NilDeal};
    let (mut w, j) = college(true);
    // The sandbox's NIL Go switch, so every dollar clears and the sums read
    // plainly.
    w.set_sandbox_switch(crate::world::sandbox::Switch::NilGo, true);
    // And no budget limit: fitting a short pot is its own test below.
    w.set_sandbox_switch(crate::world::sandbox::Switch::NilBudget, true);
    let usual: Vec<(u32, f64)> = usual_deals(&w, j);
    let at = |w: &World, id: u32| w.people.iter().position(|p| p.player.id == id).unwrap();
    // Three men staying with seasons left (a man leaving keeps the split;
    // the seat's roster is filled with former pros, whose clock has run).
    let label = if w.today < w.dates.fall_camp { w.year - 1 } else { w.year };
    let ids: Vec<u32> = usual.iter().map(|x| x.0).take(3).collect();
    for &id in &ids {
        let x = at(&w, id);
        let p = &mut w.people[x];
        p.class_of = label - 1;
        p.college_seasons = 1;
        p.cal.count_pending = false;
        p.cal.declare_plan = None;
        p.cal.portal = None;
        p.cal.nil_school = 0.0;
        let div = w.programs[j].division;
        assert!(!crate::world::college::no_season_left(&w.people[x], div, label), "a season left");
        assert!(w.people[x].cut_here().is_none());
    }
    let (star, signed, rest) = (ids[0], ids[1], ids[2]);
    // No season's play to read: no rival bids, so the sums are plain.
    for id in [star, signed, rest] {
        let x = at(&w, id);
        w.people[x].last_performance = crate::player::OptF64::NONE;
        w.people[x].cal.nil_deal = None;
        w.people[x].cal.nil_paid = 0.0;
        w.people[x].nil_income = 0.0;
    }
    // The star: $4.2M from the school and collective last July.
    let i = at(&w, star);
    w.people[i].cal.nil_paid = 4.2e6;
    w.people[i].brand = 300_000.0;
    w.people[i].nil_income = 4.5e6;
    // A transfer who signed for the coming money year.
    let k = at(&w, signed);
    let year = deals_year(&w);
    w.people[k].cal.nil_deal = Some(NilDeal { year, amount: 123_000.0, kind: DealKind::Transfer, through: 0 });
    // The split as it stands once his deal is signed: a deal that pays is
    // fixed, and the rest of the roster shares what it leaves.
    let usual: Vec<(u32, f64)> = usual_deals(&w, j);
    let coll: std::collections::HashMap<u32, f64> = collective_deals(&w, j).into_iter().map(|x| (x.0, x.1)).collect();
    let split = |id: u32| usual.iter().find(|x| x.0 == id).unwrap().1 + coll.get(&id).copied().unwrap_or(0.0);
    assert!(w.front_office.college.deals.is_empty() && !w.front_office.college.seeded);
    ready(&mut w);
    let pay = |w: &World, id: u32| deal_of(w, id).map(|d| d.0 + d.1).unwrap();
    assert!((pay(&w, star) - 4.2e6).abs() <= 1_000.0, "his pay last July, brand apart: {}", pay(&w, star));
    assert!((pay(&w, signed) - 123_000.0).abs() <= 1_000.0, "what he signed for: {}", pay(&w, signed));
    let r = pay(&w, rest);
    assert!(r <= split(rest) + 1_000.0, "the rest keep at most the split: {r} against {}", split(rest));
    // The market price never reads a player without a season's play.
    assert!(market_prices(&w, j).values().all(|&x| x > 0.0));
    let desk = &w.front_office.college;
    assert!(desk.inherited.contains(&star) && desk.inherited.contains(&rest), "marked as the old staff's");
    // He feels paid at the old staff's number, not cheated: last July's pay
    // is his bar, without his own brand money.
    let (s, c) = deal_of(&w, star).unwrap();
    let now = (w.today, w.played);
    set_deal(&mut w, star, Some(s), Some(c)).unwrap();
    to_next_section(&mut w);
    let r = reaction(&w, star, MoneyKind::Deal).unwrap();
    assert!(!matches!(r.mood, Mood::Insulted | Mood::WantsMore), "{:?}: {}", r.mood, r.text);
    assert!(!w.front_office.college.inherited.contains(&star), "your number now");
    // A cut from it is heard as one.
    (w.today, w.played) = now;
    set_deal(&mut w, star, Some(s / 4.0), Some(c / 4.0)).unwrap();
    to_next_section(&mut w);
    let r = reaction(&w, star, MoneyKind::Deal).unwrap();
    assert!(r.text.contains("cut from"), "{}", r.text);
}

/// The inherited deals fitted to a pot (Codex's review of #544): signed
/// deals stand, the market raises shrink first, then every split.
#[test]
fn inherited_deals_fit_the_pot_raises_first() {
    use crate::world::front_office::college::{Priced, fit};
    let rows = [(Priced::Signed, 100.0, 10.0), (Priced::Market, 300.0, 100.0), (Priced::Split, 50.0, 50.0)];
    assert_eq!(fit(1_000.0, false, &rows), vec![100.0, 300.0, 50.0], "room for all");
    assert_eq!(fit(10.0, true, &rows), vec![100.0, 300.0, 50.0], "no limit");
    // 350 left: the raise of 200 halves to 100.
    assert_eq!(fit(350.0, false, &rows), vec![100.0, 200.0, 50.0]);
    // 175 left: raises gone, the splits (100 + 50) take half.
    assert_eq!(fit(175.0, false, &rows), vec![100.0, 50.0, 25.0]);
    // Less than the signed deal: only it stands.
    assert_eq!(fit(50.0, false, &rows), vec![100.0, 0.0, 0.0]);
}

/// 2026-11-18: your desk's roster split is the split an AI school's July 1
/// makes (before, a flat 0.6 of it): at an AI program, every player
/// without a deal is paid exactly the split read the day before.
#[test]
fn the_roster_split_is_what_an_ai_school_pays() {
    use crate::world::front_office::college::splits_for;
    let (mut w, j) = college(true);
    w.rebuild_rosters();
    let k =
        (0..w.programs.len()).find(|&k| k != j && w.programs[k].division == crate::world::data::Division::FBS).unwrap();
    // A world with no history has no college players: young pros go back
    // to the AI school too, as `college` does for yours.
    let pros: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::Pro).take(60).collect();
    for (n, &i) in pros.iter().enumerate() {
        let p = &mut w.people[i];
        p.stage = Stage::College;
        p.pro = None;
        p.college = Some(k);
        p.college_seasons = 1;
        p.last_role = Some(match n % 3 {
            0 => crate::player::PlayingTime::Starter,
            1 => crate::player::PlayingTime::Rotation,
            _ => crate::player::PlayingTime::Backup,
        });
    }
    w.rebuild_rosters();
    let c = &mut w.programs[k].collective;
    c.budget = 20.0e6;
    c.school = 12.0e6;
    let year = w.today.year();
    let split: std::collections::HashMap<u32, f64> =
        splits_for(&w, k, year).into_iter().map(|x| (x.0, x.1 + x.2)).collect();
    let free: Vec<u32> = w.programs[k]
        .roster
        .iter()
        .map(|&i| &w.people[i])
        .filter(|p| p.cal.nil_deal.is_none_or(|d| !d.pays(year)))
        .map(|p| p.player.id)
        .collect();
    assert!(free.len() > 20, "{}", free.len());
    rec::nil_payouts(&mut w);
    let mut paid_some = 0;
    for id in free {
        let p = w.people.iter().find(|p| p.player.id == id).unwrap();
        let s = split.get(&id).copied().unwrap_or(0.0);
        assert!((p.cal.nil_paid as f64 - s).abs() <= 1.0 + 1e-6 * s, "{id}: paid {} against {s}", p.cal.nil_paid);
        paid_some += (s > 1_000.0) as usize;
    }
    assert!(paid_some > 10, "{paid_some}");
}

/// One season's row for a test player.
fn season_row(level: crate::player::Level, role: crate::player::PlayingTime, play: f64) -> crate::player::SeasonRecord {
    use crate::player::OptF64;
    crate::player::SeasonRecord {
        position: crate::positions::Position::WR,
        age: 19,
        level,
        playing_time: role,
        overall_start: 60.0,
        overall: 60.0,
        potential: 60.0,
        performance: OptF64::some(play),
        level_rating: None,
        level_rating_end: None,
        coaching: 10.0,
        events: Default::default(),
        games_missed: 0,
        adjustment: OptF64::NONE,
        spring_gain: OptF64::NONE,
        spring_open: OptF64::NONE,
        team: crate::player::OptIdx::NONE,
        depth: crate::player::DepthLabel::NONE,
        situation: OptF64::NONE,
        results: OptF64::NONE,
    }
}

/// The third party's refit (2026-11-25): the third party's pot fills every
/// man from his revenue share up to the same multiple of his market price.
#[test]
fn the_third_party_fills_each_man_to_his_market_price() {
    // The school pays three men 100, 300 and 50; the market prices them at
    // 400, 200 and nothing.
    let school = [100.0, 300.0, 50.0];
    let price = [400.0, 200.0, 0.0];
    // A pot of 300 takes the first to his price. The second is already
    // paid over his, and nobody prices the third.
    let c = rec::fill_to_market(&school, &price, 300.0);
    assert!((c[0] - 300.0).abs() < 1e-6 && c[1] == 0.0 && c[2] == 0.0, "{c:?}");
    // A smaller pot stops short of it.
    let c = rec::fill_to_market(&school, &price, 150.0);
    assert!((c[0] - 150.0).abs() < 1e-6 && c[1] == 0.0, "{c:?}");
    // A bigger one lifts both to the same multiple of their prices.
    let c = rec::fill_to_market(&school, &price, 700.0);
    let mu = 1100.0 / 600.0;
    assert!((c[0] - (mu * 400.0 - 100.0)).abs() < 1e-6, "{c:?}");
    assert!((c[1] - (mu * 200.0 - 300.0)).abs() < 1e-6, "{c:?}");
    assert!((c.iter().sum::<f64>() - 700.0).abs() < 1e-6 && c[2] == 0.0);
    // No pot, or nobody priced: nothing.
    assert!(rec::fill_to_market(&school, &price, 0.0).iter().all(|&x| x == 0.0));
    assert!(rec::fill_to_market(&school, &[0.0; 3], 100.0).iter().all(|&x| x == 0.0));
}

/// The third party's refit (2026-11-25): the starters' bars read only men
/// who started a season at their school, and July 1 reads each man in the
/// role his school's depth chart gives him today.
#[test]
fn july_reads_real_starters_and_the_depth_chart() {
    use crate::player::{Level, OptF64, PlayingTime as R};
    let (mut w, j) = college(true);
    let div = w.programs[j].division;
    let i = w.programs[j].roster[0];
    {
        let p = &mut w.people[i];
        p.cal.nil_role = None;
        p.last_role = Some(R::Starter);
        p.last_performance = OptF64::some(70.0);
        p.player.history = vec![season_row(Level::HS, R::Starter, 70.0)];
    }
    // A newcomer who started in high school is on no starters' bar.
    assert!(!rec::level_starter(&w.people[i]));
    // With a college season he is; a transfer who landed isn't until he
    // has played where he is.
    w.people[i].player.history = vec![season_row(Level::FBS, R::Starter, 70.0)];
    assert!(rec::level_starter(&w.people[i]));
    w.people[i].cal.nil_role = Some((R::Rotation, 0.0));
    assert!(!rec::level_starter(&w.people[i]));
    w.people[i].cal.nil_role = None;
    // His play as it is, in the role the chart gives him.
    let medians = rec::nil_medians(&w);
    let last = rec::nil_read(&w.people[i], div, &medians);
    assert_eq!(last.0, Some(R::Starter));
    assert_eq!(rec::market_read(&w.people[i], div, &medians, Some(R::Rotation)), (Some(R::Rotation), last.1));
    assert_eq!(rec::market_read(&w.people[i], div, &medians, None), last, "off the chart: his last role");
    // A man with no college season is a newcomer whatever the chart says.
    w.people[i].player.history = vec![season_row(Level::HS, R::Starter, 70.0)];
    assert_eq!(rec::market_read(&w.people[i], div, &medians, Some(R::Starter)).0, None);
    // The chart is the one fall camp will set: every roster man is on it.
    let chart = crate::world::season::program_chart(&w, &w.programs[j], &w.seat_order());
    assert_eq!(chart.len(), w.programs[j].roster.len());
    assert_eq!(rec::chart_role(&w, j, &w.people[i]), chart.iter().find(|x| x.0 == i).map(|x| x.1));
}

/// The third party's refit (2026-11-25): in two pots a school's July split
/// pays every man his revenue share and the third party's money on top, up
/// to one multiple of his market price; a man with no college season and
/// no deal gets no third-party money.
#[test]
fn the_july_split_pays_the_chart_up_to_its_market() {
    use crate::player::{Level, OptF64, PlayingTime as R};
    use crate::world::nil_bid;
    let (mut w, j) = college(true);
    let roster = w.programs[j].roster.clone();
    assert!(roster.len() >= 20, "{}", roster.len());
    // Everyone a scholarship man with a college season, but the last five,
    // who have only a high school one.
    for (n, &i) in roster.iter().enumerate() {
        let p = &mut w.people[i];
        let new = n + 5 >= roster.len();
        let play = 55.0 + (n % 12) as f64;
        p.walk_on = false;
        p.cal.nil_role = None;
        p.cal.nil_deal = None;
        p.last_role = Some(if n % 2 == 0 { R::Starter } else { R::Backup });
        p.last_performance = OptF64::some(play);
        p.player.history = vec![season_row(if new { Level::HS } else { Level::FBS }, R::Starter, play)];
    }
    let medians = rec::nil_medians(&w);
    let bars = nil_bid::ShareBars::read(&w);
    let level = nil_bid::level(&w.programs[j]);
    // The chart: the first third start, the next third rotate.
    let chart: Vec<(usize, R)> = roster
        .iter()
        .enumerate()
        .map(|(n, &i)| (i, [R::Starter, R::Rotation, R::Backup][(3 * n / roster.len()).min(2)]))
        .collect();
    let year = w.today.year();
    // $20M a year, a fifth of it the school's.
    let rows =
        rec::roster_split(&w, j, &roster, 20.0e6, 0.2, true, &medians, &bars, year, &chart, |_, _| rec::NilHeld::None);
    // Both pots are paid in full.
    let school: f64 = rows.iter().map(|r| r.school).sum();
    let coll: f64 = rows.iter().map(|r| r.coll).sum();
    assert!((school - 4.0e6).abs() < 1.0, "{school}");
    assert!((coll - 16.0e6).abs() < 1.0, "{coll}");
    let mut multiple: Option<f64> = None;
    let mut paid = 0;
    for (n, r) in rows.iter().enumerate() {
        let p = &w.people[r.i];
        if n + 5 >= roster.len() {
            assert_eq!(r.role, None);
            assert_eq!(r.coll, 0.0, "a newcomer with no deal signed for the scholarship");
            continue;
        }
        // Read on the chart, not on last season's role.
        assert_eq!(r.role, Some(chart[n].1));
        let price = 20.0e6 * nil_bid::market_unit_on(&bars, level, p, r.role, r.edge);
        if r.coll > 0.0 {
            // Everyone the third party pays sits at the same multiple of
            // his price, school's share and all.
            let mu = (r.school + r.coll) / price;
            let m = *multiple.get_or_insert(mu);
            assert!((mu - m).abs() < 1e-6 * m, "{mu} against {m}");
            paid += 1;
        } else if let Some(m) = multiple {
            // The rest are paid over it by the school already, or sit
            // outside the paid set as deep backups.
            assert!(r.school >= m * price - 1.0 || !r.paid, "{} under {m} x {price}", r.school);
        }
    }
    assert!(paid >= 3, "{paid}");
    // A deal that stands takes its money first; the rest of the year's
    // money goes to the rest, and the holder's row still carries his own
    // split of it, the going rate a desk shows beside his deal.
    let star = roster[0];
    let held = rec::roster_split(&w, j, &roster, 20.0e6, 0.2, true, &medians, &bars, year, &chart, |_, i| {
        if i == star { rec::NilHeld::Fixed(3.0e6) } else { rec::NilHeld::None }
    });
    assert_eq!(held[0].fixed, Some(3.0e6));
    assert!(held[0].school + held[0].coll > 0.0, "a deal holder's own split");
    let rest: f64 = held.iter().filter(|r| r.fixed.is_none()).map(|r| r.school + r.coll).sum();
    assert!((rest - 17.0e6).abs() < 1.0, "{rest}");
}

/// 2026-11-18: a recruit's stars are read at the school's money level, its
/// starters' stars under the Power 4's (the Group of 5 signee fix).
#[test]
fn a_recruits_stars_read_at_the_schools_level() {
    use crate::world::nil_bid::{star_at, star_bid, star_bid_at, star_lifts};
    for s in 1..=5u8 {
        assert!((star_bid_at(s as f64) - star_bid(s)).abs() < 1e-12, "{s}");
    }
    assert!(star_bid_at(3.5) > star_bid(3) && star_bid_at(3.5) < star_bid(4));
    let (mut w, _) = college(true);
    let lift = star_lifts(&w);
    assert_eq!(lift[0], 0.0, "the scale is the Power 4's");
    assert!(lift.iter().all(|&x| (0.0..3.0).contains(&x)), "{lift:?}");
    w.nil_book.star_lift = [0.0, 1.0, 1.5, 2.0];
    assert!((star_at(&w, 0, 3) - star_bid(3)).abs() < 1e-12);
    assert!((star_at(&w, 1, 3) - star_bid(4)).abs() < 1e-12, "a Group of 5 three-star as its own four-star");
    assert!((star_at(&w, 1, 5) - star_bid(5)).abs() < 1e-12, "never past a five-star");
}
