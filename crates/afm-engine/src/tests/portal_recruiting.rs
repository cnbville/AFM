//! Portal recruiting (Cain 2026-11-07): a transfer gets the options a high
//! school recruit has (a call with its pitches and a promise, a campus or
//! home visit, the offer), and a player your offer satisfies decides now
//! instead of waiting for his date or his wave (the satisfaction bar).

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::clock::PortalWindow;
use crate::world::data::Division;
use crate::world::folk::scene;
use crate::world::front_office::{college as desk, portal};
use crate::world::{PortalEntry, Stage, World, recruiting};

/// Your FBS program, and a good starter at another FBS program in the
/// portal, deciding in ten days.
fn entrant() -> (World, usize, usize) {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(20);
    let i = enter(&mut w, mine, true);
    (w, mine, i)
}

/// One more player at another FBS program, in the portal from today and
/// deciding in ten days (the window opened the first time).
fn enter(w: &mut World, mine: usize, open: bool) -> usize {
    let q = (0..w.programs.len()).find(|&q| q != mine && w.programs[q].division == Division::FBS).unwrap();
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let p = &mut w.people[i];
    p.stage = Stage::College;
    p.college = Some(q);
    p.path.push(q);
    p.class_of = year - 2;
    p.college_seasons = 2;
    p.last_role = Some(PlayingTime::Backup);
    p.cal.lean = 1.0;
    w.programs[q].roster.push(i);
    // He wants out, so the window takes him on its first day.
    if open {
        let close = w.today.add(20);
        recruiting::portal_open(w, PortalWindow::Winter, close);
    }
    let today = w.today;
    assert!(w.portal_market.is_some(), "the window opened a market");
    let e = w.people[i].cal.portal.get_or_insert(PortalEntry {
        entered: today,
        decide_by: today,
        why: "playing time",
        held: false,
        looking: false,
    });
    e.entered = today;
    e.held = false;
    e.decide_by = today.add(10);
    if !w.portal_pending.contains(&i) {
        w.portal_pending.push(i);
    }
    i
}

#[test]
fn a_transfer_takes_your_call_and_your_promise() {
    let (mut w, _, i) = entrant();
    let id = w.people[i].player.id;
    // A call needs no offer (and an offer alone may satisfy him already).
    let s = portal::call(&mut w, id).expect("a call to an entrant");
    assert!(portal::call(&mut w, id).is_err(), "one call a day");
    let sc = scene::open(&mut w, s).unwrap();
    assert!(sc.title.starts_with("A call to"), "{}", sc.title);
    assert!(sc.lines.iter().any(|l| !l.text.is_empty() && l.by == 2), "he asks first, in his own words");
    // Listen, then make a point, then promise him a starting job.
    let sc = scene::answer(&mut w, s, 709).unwrap();
    // His question may be about a group you have nothing in: your points
    // are all behind "bring up something else".
    let point =
        sc.picks.iter().chain(&sc.reach).find(|p| (7100..7300).contains(&p.code)).expect("a point to make").code;
    let sc = scene::answer(&mut w, s, point).unwrap();
    assert!(sc.picks.iter().any(|p| p.code == 7520), "a promise is on the table after the pitch");
    let sc = scene::answer(&mut w, s, 7520).unwrap();
    assert!(sc.picks.iter().any(|p| p.code == 711 && p.kind == scene::PickKind::Final), "a promise binds you");
    let sc = scene::answer(&mut w, s, 711).unwrap();
    assert!(sc.done);
    let t = w.front_office.college.portal.iter().find(|t| t.id == id).unwrap();
    assert_eq!(t.talks.len(), 1, "the call is on his file");
    assert_eq!(t.talks[0].at, 0);
    assert_eq!(t.promise.map(|p| p.0), Some(0));
}

#[test]
fn he_visits_your_campus_and_the_sit_down_waits_for_you() {
    let (mut w, mine, i) = entrant();
    let id = w.people[i].player.id;
    desk::portal_offer(&mut w, id, None, None).unwrap();
    let days = portal::visit_dates(&w, id, desk::VisitKind::Official);
    let Some(&(d, _)) = days.first() else {
        // A dead period covers his whole window: no visit, by the rules.
        let d = w.today.add(1);
        assert!(portal::set_visit(&mut w, id, desk::VisitKind::Official, d).is_err());
        return;
    };
    match portal::set_visit(&mut w, id, desk::VisitKind::Official, d) {
        Ok(_) => {
            assert!(
                w.folk.scenes.iter().any(|s| s.kind == scene::Kind::PortalVisit && s.a == id as u64 && s.date == d)
            );
            assert!(portal::set_visit(&mut w, id, desk::VisitKind::Official, d).is_err(), "one campus visit");
            w.today = d;
            desk::day(&mut w, d);
            let t = w.front_office.college.portal.iter().find(|t| t.id == id).unwrap();
            if w.people[i].cal.portal.is_some() {
                assert!(t.feel.is_some(), "the visit happened");
            }
            let _ = mine;
        }
        // He may keep his trips for others: an answer, in words.
        Err(t) => assert!(t.contains("said no"), "{t}"),
    }
}

#[test]
fn an_offer_he_cant_refuse_decides_him_now() {
    let (mut w, mine, i) = entrant();
    let id = w.people[i].player.id;
    let today = w.today;
    let left = desk::budget(&w, mine).left.max(0.0);
    let said = desk::portal_offer(&mut w, id, Some(left.min(4.0e6)), None).unwrap();
    let t = w.front_office.college.portal.iter().find(|t| t.id == id).unwrap();
    assert!(t.committed.is_some(), "a clear overpay should decide him on the spot: {said}");
    assert_eq!(w.people[i].cal.portal.unwrap().decide_by, today, "he decides tonight, not on his date");
    recruiting::portal_day(&mut w, false);
    assert_eq!(w.people[i].college, Some(mine), "he signs with you");
}

#[test]
fn his_bar_is_his_own() {
    let (mut w, _, i) = entrant();
    let a = portal::bar(&w, i);
    // A player who loves being courted needs more.
    w.people[i].player.personality.traits.ego = 20;
    w.people[i].player.personality.traits.loyalty = 1;
    let b = portal::bar(&w, i);
    assert!(b > a, "{b} not above {a}");
    // And it falls as his date nears.
    w.today = w.people[i].cal.portal.unwrap().decide_by;
    assert!(portal::bar(&w, i) < b);
}

#[test]
fn a_recruit_your_offer_satisfies_commits_before_his_wave() {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let class = recruiting::next_class(&w);
    let i = (0..w.people.len())
        .find(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].cal.offers.is_empty())
        .unwrap();
    let p = &mut w.people[i];
    p.class_of = class;
    // A kid who knows his mind: loyal, no ego, in love with the game.
    p.cal.commit_timing = Some(crate::world::CommitTiming::Summer(0));
    let t = &mut p.player.personality.traits;
    (t.ego, t.ambition, t.loyalty) = (1, 1, 20);
    p.player.personality.love_of_game = 20.0;
    let id = p.player.id;
    assert!(desk::hs_bar(&w, i) < 0.06, "his bar is low");
    let said = desk::offer(&mut w, id).unwrap();
    assert_eq!(w.people[i].cal.commit.map(|c| c.0), Some(mine), "he commits on the spot: {said}");
    assert!(said.contains("commits to you"), "{said}");
    assert!(w.people[i].cal.commit_timing <= Some(crate::world::CommitTiming::Early), "he signs in December");
    // A kid who waits for February and enjoys the attention holds out.
    let j = (0..w.people.len())
        .find(|&j| w.people[j].stage == Stage::HighSchool && w.people[j].cal.offers.is_empty() && j != i)
        .unwrap();
    let p = &mut w.people[j];
    p.class_of = class;
    p.cal.commit_timing = Some(crate::world::CommitTiming::Late);
    let t = &mut p.player.personality.traits;
    (t.ego, t.ambition, t.loyalty) = (20, 20, 1);
    let id = p.player.id;
    desk::offer(&mut w, id).unwrap();
    assert!(w.people[j].cal.commit.is_none(), "a lone offer doesn't clear a high bar");
}

#[test]
fn a_yes_holds_his_place_on_your_roster() {
    let (mut w, mine, a) = entrant();
    let b = enter(&mut w, mine, false);
    // One place left on your roster.
    let season = w.portal_market.as_ref().unwrap().season;
    let limit = crate::world::ncaa::roster_limits(&w.programs[mine], season).0;
    let year = w.year;
    while recruiting::counted_rosters(&w, season)[mine] + 1 > limit {
        w.programs[mine].roster.pop();
    }
    // The test world's rosters are thin: walk the class's kids onto yours.
    let kids: Vec<usize> = (0..w.people.len()).filter(|&k| w.people[k].stage == Stage::HighSchool).collect();
    let mut kids = kids.into_iter();
    while recruiting::counted_rosters(&w, season)[mine] + 1 < limit {
        let k = kids.next().expect("enough kids to fill a roster");
        let p = &mut w.people[k];
        (p.stage, p.college, p.class_of, p.college_seasons, p.walk_on) =
            (Stage::College, Some(mine), year - 1, 1, false);
        p.path.push(mine);
        w.programs[mine].roster.push(k);
    }
    assert_eq!(recruiting::counted_rosters(&w, season)[mine] + 1, limit, "one place open");
    // B played better, so he would decide first on a usual night.
    w.people[b].last_performance = Some(w.people[a].last_performance.unwrap_or(60.0) + 10.0);
    let (ia, ib) = (w.people[a].player.id, w.people[b].player.id);
    let left = desk::budget(&w, mine).left.max(0.0);
    let said = desk::portal_offer(&mut w, ia, Some((left * 0.5).min(4.0e6)), None).unwrap();
    let committed = |w: &World, id| w.front_office.college.portal.iter().any(|t| t.id == id && t.committed.is_some());
    assert!(committed(&w, ia), "a clear overpay decides him: {said}");
    // The last place is his: B can't tell you yes too.
    let said = desk::portal_offer(&mut w, ib, Some((left * 0.5).min(4.0e6)), None).unwrap();
    assert!(!committed(&w, ib), "your roster has no place left for a second yes: {said}");
    assert!(!portal::race(&w, b).iter().any(|r| r.0 == mine), "you aren't in his race");
    // Without A's yes, the place would be open to him.
    let t = w.front_office.college.portal.iter_mut().find(|t| t.id == ia).unwrap();
    let yes = t.committed.take();
    assert!(portal::race(&w, b).iter().any(|r| r.0 == mine), "with A's place free, you are in B's race");
    w.front_office.college.portal.iter_mut().find(|t| t.id == ia).unwrap().committed = yes;
    assert!(desk::portal_pull(&mut w, ia).is_err(), "the offer he said yes to holds");
    // Both decide tonight: the one who said yes signs first.
    w.people[b].cal.portal.as_mut().unwrap().decide_by = w.today;
    recruiting::portal_day(&mut w, false);
    assert_eq!(w.people[a].college, Some(mine), "he signs with you");
    assert_ne!(w.people[b].college, Some(mine), "no place was left for him");
}

#[test]
fn a_playoff_teams_entrant_takes_your_call_once_his_team_is_out() {
    // Minor bug 373: a playoff team's entrant keeps `held` for the record
    // after his team is out and his real day is set. Your call, your visits
    // and your offer's early yes all read that flag as "still playing", so he
    // couldn't be called, visited or say yes until his decision day.
    let (mut w, mine, i) = entrant();
    let id = w.people[i].player.id;
    let today = w.today;
    w.people[i].cal.portal.as_mut().unwrap().held = true;
    assert!(!w.people[i].cal.portal.unwrap().on_hold(), "released: he has his day");
    assert!(!portal::visit_dates(&w, id, desk::VisitKind::Official).is_empty(), "his visits can be set");
    let shut = portal::visit_rule(&w, i, desk::VisitKind::Home, today.add(1));
    assert_ne!(shut.as_deref(), Some("He isn't shopping in the portal."), "a visit is open to him");
    portal::call(&mut w, id).expect("a call to an entrant whose team is out");
    let left = desk::budget(&w, mine).left.max(0.0);
    let said = desk::portal_offer(&mut w, id, Some(left.min(4.0e6)), None).unwrap();
    let t = w.front_office.college.portal.iter().find(|t| t.id == id).unwrap();
    assert!(t.committed.is_some(), "a clear overpay decides him on the spot: {said}");
    // Still on hold (his team plays on): no day, no call, no yes.
    let (mut w, mine, i) = entrant();
    let id = w.people[i].player.id;
    let e = w.people[i].cal.portal.as_mut().unwrap();
    (e.held, e.entered, e.decide_by) = (true, recruiting::HELD, recruiting::HELD);
    assert!(w.people[i].cal.portal.unwrap().on_hold());
    let said = portal::call(&mut w, id).unwrap_err();
    assert!(said.contains("his team is still playing"), "{said}");
    assert!(portal::race(&w, i).is_empty());
    let left = desk::budget(&w, mine).left.max(0.0);
    let _ = desk::portal_offer(&mut w, id, Some(left.min(4.0e6)), None);
    assert!(
        w.front_office.college.portal.iter().find(|t| t.id == id).is_none_or(|t| t.committed.is_none()),
        "he says nothing until his team is out"
    );
}
