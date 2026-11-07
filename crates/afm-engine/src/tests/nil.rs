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
    let (id, usual) = usual_deals(&w, j)[0];
    // Every player of yours starts on the going rate (Cain 2026-11-07:
    // every NIL dollar is yours, so nobody is left to a split).
    fill_deals(&mut w);
    let b = budget(&w, j);
    assert!(b.two_pots);
    assert!((b.yours - 12.0e6).abs() < 1.0, "the school's pot is yours: {}", b.yours);
    assert!((b.collective - 8.0e6).abs() < 1.0);
    assert!(b.collective_roster > 0.0 && b.collective_roster <= 0.6 * 8.0e6 + 1.0, "{}", b.collective_roster);
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
    let filled = set_usual_deals(&mut w).unwrap();
    assert!(filled > 0, "{filled}");
    let now = usual_deals(&w, j).into_iter().find(|d| d.0 == id).unwrap().1;
    assert!((deal_of(&w, id).unwrap().0 - now).abs() <= 250.0 && usual > 0.0);
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
    let id = w.people[i].player.id;
    let usual = rec::usual_nil_offer(&w, j, i);
    assert!(usual > 1_000.0, "{usual}");
    let commit = w.people[i].cal.commit;
    let text = set_promise(&mut w, id, Some(10.0 * usual + 1.0e6), None).unwrap();
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
        let top = rec::nil_go_top_recruit(&w.people[i]);
        let c = rec::nil_go(w.today, collective.max(10_000.0), top);
        assert!((weighed_offer(&w, j, id) - 50_000.0 - c).abs() < 1.0, "{} against {c}", weighed_offer(&w, j, id));
    } else {
        assert!(set_promise(&mut w, id, Some(50_000.0), Some(10_000.0)).is_err(), "one pot before July 2025");
    }
    // With your coordinator on the board his offers carry the usual money.
    set_promise(&mut w, id, None, None).unwrap();
    w.front_office.set_owner(Duty::Board, Owner::Staff);
    let usual = rec::usual_nil_offer(&w, j, i);
    assert!((weighed_offer(&w, j, id) - usual).abs() < 1.0, "{} against {usual}", weighed_offer(&w, j, id));
    assert!(stage_board(&w).unwrap().offers.iter().find(|o| o.person == i).unwrap().nil.is_none());
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
