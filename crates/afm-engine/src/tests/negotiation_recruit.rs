//! The negotiation overhaul, lane NG3 (`docs/design/negotiation-arguments.md`,
//! item 7): a true point to a recruit about his film or his place on your
//! depth chart takes your own bid out of his money cap, so it lowers his
//! number only as far as his other offers see him; a humble, coachable kid
//! takes it straight, most take it badly.

use super::persona_deals::{college, fresh, wants};
use crate::world::folk::claims::Stated;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::deals::Deals;
use crate::world::folk::persona::fake::Fake;
use crate::world::folk::persona::negotiate::points::{self, RecruitPoint};
use crate::world::folk::persona::terms::{Give, Offer, Valuing, Weighed};
use crate::world::folk::persona::wants::WantKind;

fn money_of(w: &Weighed) -> Option<f64> {
    let Weighed::Counter(c) = w else { return None };
    c.gives.iter().find_map(|g| if let Give::Money(x) = g { Some(*x) } else { None })
}

#[test]
fn a_true_point_caps_his_money_at_what_his_other_offers_bid() {
    let (mut w, p) = college();
    let other = (0..w.programs.len()).find(|&j| j != p && w.programs[j].division == w.programs[p].division).unwrap();
    let f = Fake { wants: wants(&[WantKind::Money, WantKind::Place]), ..Default::default() };
    let d = Deals { wants: &f };
    let small = Offer { gives: vec![Give::Scholarship(None), Give::Money(1000.0)], ..Default::default() };
    // You can pay far more than his other school.
    w.programs[p].collective.budget = 40.0e6;
    w.programs[other].collective.budget = 2.0e6;
    let mut used = Vec::new();
    let mut seen = 0;
    for _ in 0..200 {
        let r = fresh(&w, &used, 17);
        used.push(r);
        w.people[r].cal.offers = vec![other];
        w.people[r].stars = 5;
        w.people[r].family.decider = crate::world::background::Decider::Himself;
        let him = Them::Player(w.people[r].player.id);
        let Some(before) = money_of(&d.weigh(&w, him, &small)) else { continue };
        let theirs = crate::world::recruiting::recruit_top(&w, other, r);
        let mine = crate::world::recruiting::recruit_top(&w, p, r);
        if theirs <= 0.0 || mine <= theirs * 1.2 || before <= theirs {
            continue;
        }
        // The point made in the same line counts for its offer.
        let argued = Offer { argued: 1.0, ..small.clone() };
        let after = d.weigh(&w, him, &argued);
        if let Some(x) = money_of(&after) {
            assert!(x <= theirs.max(1000.0) + 1000.0, "capped at his other offer: {x} over {theirs}");
            assert!(x < before, "{x} {before}");
        }
        seen += 1;
        break;
    }
    assert!(seen > 0, "no recruit whose own number sat over his other offers");
}

#[test]
fn his_film_and_his_place_are_read_and_he_takes_it_by_his_makeup() {
    let (mut w, p) = college();
    let r = fresh(&w, &[], 17);
    let say = |v: &str| Stated { kind: "stat".into(), value: v.into(), ..Default::default() };
    let (kind, _) =
        points::recruit_read(&w, r, p, &say("your film doesn't look like your stars")).expect("a film point");
    assert_eq!(kind, RecruitPoint::Film);
    let (kind, true_) = points::recruit_read(&w, r, p, &say("you'd be behind two guys")).expect("a depth point");
    assert_eq!(kind, RecruitPoint::Depth);
    assert_eq!(true_, crate::world::recruiting::ahead_of_recruit(&w, p, r) > 0);
    assert!(points::recruit_read(&w, r, p, &say("you're a great kid")).is_none());
    // A proud kid takes it badly; a humble, coachable one straight.
    w.people[r].player.personality.traits.ego = 20;
    assert!(points::recruit_land(&mut w, r, RecruitPoint::Depth) < 0.0);
    assert!(points::recruit_landed(&w, w.people[r].player.id));
    // Heard once already: nothing more.
    assert_eq!(points::recruit_land(&mut w, r, RecruitPoint::Depth), 0.0);
    let k = fresh(&w, &[r], 17);
    w.people[k].player.personality.traits.ego = 1;
    w.people[k].player.personality.coachability = 18.0;
    assert!(points::recruit_land(&mut w, k, RecruitPoint::Film) > 0.0);
}
