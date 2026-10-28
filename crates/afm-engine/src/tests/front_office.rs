//! Your front office (lane 10): your choices go through the market's own
//! code, the rules hold for you as for the AI, and what you hand to your
//! staff is decided the AI's way.

use crate::world::front_office::{self as fo, Duty, Owner, deals, nfl, roster};
use crate::world::pro::{self, Status, money};
use crate::world::seat::{Job, You};
use crate::world::{Stage, World, WorldOptions};

/// A tiny world in the 2025 league year with you as an NFL head coach and
/// general manager.
fn nfl_world() -> (World, usize) {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    assert_eq!(w.year, 2025);
    let t = (0..w.clubs.len())
        .filter(|&t| crate::world::franchise::active(t, w.year))
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

/// Your players whose deals end before this league year.
fn expiring(w: &World, t: usize, ufa: bool) -> Vec<usize> {
    let year = w.year;
    w.clubs[t]
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            let c = w.people[i].pro.as_ref().unwrap();
            c.contract.as_ref().is_some_and(|k| k.team == t && k.last_year() < year)
                && (!ufa || c.accrued >= money::ufa_seasons(year))
        })
        .collect()
}

/// Gives a player a deal that ended last season, so he is expiring.
fn make_expiring(w: &mut World, t: usize, i: usize, accrued: u32) {
    let year = w.year;
    let apy = 6.0e6;
    let c = w.people[i].pro.as_mut().unwrap();
    c.accrued = accrued;
    c.contract = Some(money::build(t, money::Kind::Veteran, year - 3, &money::Terms::flat(3, apy)));
}

#[test]
fn only_your_club_and_only_what_you_keep_is_yours() {
    let (mut w, t) = nfl_world();
    assert_eq!(fo::club(&w), Some(t));
    assert!(fo::yours(&w, t, Duty::Contracts) && fo::yours(&w, t, Duty::Tags) && fo::yours(&w, t, Duty::Trades));
    // The cap manager, the waiver desk, the squad and NIL start with your staff.
    assert!(!fo::yours(&w, t, Duty::Cap) && !fo::yours(&w, t, Duty::Waivers));
    let other = (0..w.clubs.len()).find(|&o| o != t).unwrap();
    assert!(!fo::yours(&w, other, Duty::Contracts));
    w.front_office.set_owner(Duty::Trades, Owner::Staff);
    assert!(!fo::yours(&w, t, Duty::Trades));
}

#[test]
fn your_offer_is_weighed_by_the_markets_own_score() {
    // Before the league year your expiring veteran hears your offer. A
    // lowball gets the agent's ask back (above what you offered); his ask
    // itself signs him, on the deal you wrote.
    let (mut w, t) = nfl_world();
    w.today = w.dates.tag_window_open;
    // A mid-priced veteran whose deal just ran out: worth well over the
    // minimum, with a number that fits under the cap.
    let i = *w.clubs[t]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster)
        .find(|&&i| (4.0e6..15.0e6).contains(&nfl::market_read(&w, i).0))
        .unwrap();
    make_expiring(&mut w, t, i, 5);
    let id = w.people[i].player.id;
    assert_eq!(nfl::negotiable(&w, i), Some(nfl::TalkKind::ReSign));
    let min = money::minimum(w.year, w.people[i].pro.as_ref().unwrap().accrued);
    let low = nfl::Proposal {
        years: 2,
        apy: min,
        gtd: 0.0,
        bonus: 0.0,
        void_years: 0,
        incentives: 0.0,
        no_trade: false,
        start: false,
        ramp: 0.06,
        low_first: false,
    };
    let a = nfl::propose(&mut w, id, low).unwrap();
    assert!(a.verdict != nfl::Verdict::Signed, "a minimum offer signed: {}", a.text);
    let ask = a.ask.expect("the agent names his number");
    assert!(ask.apy >= min, "the ask {} under the minimum", ask.apy);
    let a = nfl::propose(&mut w, id, ask).unwrap();
    assert_eq!(a.verdict, nfl::Verdict::Signed, "his own ask didn't sign: {}", a.text);
    let k = w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap();
    assert!(k.team == t && k.last_year() >= w.year && (k.apy() - ask.apy).abs() < 0.02 * ask.apy, "the deal changed");
}

#[test]
fn one_tag_a_club_a_year_at_the_tags_price() {
    let (mut w, t) = nfl_world();
    w.today = w.dates.tag_window_open;
    let mut list = expiring(&w, t, true);
    while list.len() < 2 {
        let i = *w.clubs[t].roster.iter().find(|i| !list.contains(i)).unwrap();
        make_expiring(&mut w, t, i, 5);
        list.push(i);
    }
    let (a, b) = (list[0], list[1]);
    let (price, _, _) = nfl::tag_prices(&w, a);
    let ida = w.people[a].player.id;
    nfl::tag(&mut w, ida, "franchise").unwrap();
    let k = w.people[a].pro.as_ref().unwrap().contract.as_ref().unwrap();
    assert_eq!(k.kind, money::Kind::Franchise);
    assert!((k.cap_hit(w.year) - price).abs() < 1.0, "the tag counts {} not {}", k.cap_hit(w.year), price);
    // From 2011 a club tags one player a year, of either kind.
    let idb = w.people[b].player.id;
    assert!(nfl::tag(&mut w, idb, "transition").is_err());
    // After the deadline nobody is tagged.
    w.today = w.dates.tag_deadline.date.add(1);
    assert!(nfl::tag(&mut w, idb, "franchise").is_err());
}

#[test]
fn a_trade_needs_the_other_gm_to_gain_on_his_own_chart() {
    let (mut w, t) = nfl_world();
    w.today = w.dates.league_year.date.add(2);
    assert!(deals::window_open(&w));
    let o = (0..w.clubs.len()).find(|&o| o != t && crate::world::franchise::active(o, w.year)).unwrap();
    let mine = deals::picks_of(&w, t);
    let theirs = deals::picks_of(&w, o);
    let first = *mine.iter().find(|p| p.round == 1).unwrap();
    let last = *mine.iter().filter(|p| p.year == first.year).max_by_key(|p| p.round).unwrap();
    let their_first = *theirs.iter().find(|p| p.round == 1 && p.year == first.year).unwrap();
    let their_last = *theirs.iter().filter(|p| p.year == first.year).max_by_key(|p| p.round).unwrap();
    let side = |picks: Vec<deals::Pick>| deals::Side { players: vec![], picks };
    // Your first for their last: he takes it.
    let good = deals::Proposal { other: o, give: side(vec![first]), get: side(vec![their_last]), conditional: None };
    assert!(deals::evaluate(&w, &good).unwrap().accept);
    // Your last for their first: no, and nothing small closes it.
    let bad = deals::Proposal { other: o, give: side(vec![last]), get: side(vec![their_first]), conditional: None };
    let a = deals::evaluate(&w, &bad).unwrap();
    assert!(!a.accept, "{}", a.text);
    // A pick you don't own can't be offered.
    let not_mine =
        deals::Proposal { other: o, give: side(vec![their_last]), get: side(vec![their_first]), conditional: None };
    assert!(deals::evaluate(&w, &not_mine).is_err());
    // Made, the picks change hands.
    let a = deals::propose(&mut w, &good).unwrap();
    assert!(a.accept);
    assert_eq!(w.market.picks.owner_of(first.year, first.round, first.orig), o);
    assert_eq!(w.market.picks.owner_of(their_last.year, their_last.round, their_last.orig), t);
    // Outside the window no trade is made.
    w.today = w.dates.trade_deadline.date.add(1);
    assert!(deals::evaluate(&w, &good).is_err());
}

#[test]
fn your_cut_plan_stands_and_the_staff_fills_the_rest() {
    // Your plan cuts the staff's best player and keeps its worst; every
    // other place goes the staff's way, and the roster lands on the limit.
    let (mut w, t) = nfl_world();
    w.rules.nfl_roster = 40;
    let (ranked, _, _) = roster::staff_cut(&w, t);
    assert!(ranked.len() > 41, "the club has {} to cut from", ranked.len());
    let best = ranked[0].0;
    let worst = ranked.last().unwrap().0;
    let (best_id, worst_id) = (w.people[best].player.id, w.people[worst].player.id);
    roster::set_plan(&mut w, best_id, Some(roster::Cut::Release)).unwrap();
    roster::set_plan(&mut w, worst_id, Some(roster::Cut::Keep)).unwrap();
    let d = w.today;
    pro::cutdown(&mut w, d);
    let st = |w: &World, i: usize| w.people[i].pro.as_ref().map(|c| (c.team, c.status));
    assert_ne!(st(&w, best).and_then(|x| x.0), Some(t), "your cut stayed");
    let (team, status) = st(&w, worst).unwrap();
    assert!(team == Some(t) && matches!(status, Status::Roster | Status::Reserve), "your keep went");
    assert_eq!(roster::count(&w, t, Status::Roster) + roster::count(&w, t, Status::Reserve), 40);
}

#[test]
fn a_released_player_leaves_his_bonus_on_your_cap() {
    let (mut w, t) = nfl_world();
    let i = *w.clubs[t]
        .roster
        .iter()
        .find(|&&i| {
            let c = w.people[i].pro.as_ref().unwrap();
            c.status == Status::Roster && c.contract.as_ref().is_some_and(|k| k.team == t && k.last_year() > w.year)
        })
        .unwrap();
    let year = w.year;
    let before = pro::space(&w, t, year);
    let k = w.people[i].pro.as_ref().unwrap().contract.clone().unwrap();
    let hit = k.cap_hit(year);
    let id = w.people[i].player.id;
    nfl::release(&mut w, id, false).unwrap();
    let c = w.people[i].pro.as_ref().unwrap();
    assert!(c.team != Some(t) || c.status == Status::FreeAgent);
    let after = pro::space(&w, t, year);
    // The room freed is his cap number less what stays as dead money.
    assert!(after - before <= hit + 1.0, "freed {} of a {} cap number", after - before, hit);
    assert!(w.people[i].stage == Stage::Pro);
}

#[test]
fn the_negotiating_window_agrees_terms_but_signs_nothing() {
    // Another club's expiring veteran in the two-day window: even a rich
    // offer stays on the table until the league year opens (real: verbal
    // agreements only, no visits, nothing signed).
    let (mut w, t) = nfl_world();
    let Some(open) = w.dates.tampering_open else { panic!("no window in 2025") };
    w.today = open.date;
    let o = (0..w.clubs.len()).find(|&o| o != t && crate::world::franchise::active(o, w.year)).unwrap();
    let i = *w.clubs[o]
        .roster
        .iter()
        .filter(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster)
        .find(|&&i| (4.0e6..15.0e6).contains(&nfl::market_read(&w, i).0))
        .unwrap();
    make_expiring(&mut w, o, i, 6);
    let id = w.people[i].player.id;
    assert_eq!(nfl::negotiable(&w, i), Some(nfl::TalkKind::FreeAgent));
    let rich = nfl::Proposal {
        years: 3,
        apy: nfl::market_read(&w, i).1 * 1.5,
        gtd: 0.6,
        bonus: 0.3,
        void_years: 0,
        incentives: 0.0,
        no_trade: false,
        start: true,
        ramp: 0.06,
        low_first: false,
    };
    let a = nfl::propose(&mut w, id, rich).unwrap();
    assert_ne!(a.verdict, nfl::Verdict::Signed, "{}", a.text);
    assert_eq!(w.people[i].pro.as_ref().unwrap().team, Some(o), "he left his club in the window");
}

#[test]
fn your_cut_plan_stands_with_the_staff_on_the_cutdown() {
    // SC6: a cut you mark with the cutdown at your staff stands, the staff
    // decides the rest, and the duty stays with the staff.
    let (mut w, t) = nfl_world();
    w.rules.nfl_roster = 40;
    w.front_office.set_owner(Duty::Cutdown, Owner::Staff);
    let (ranked, _, _) = roster::staff_cut(&w, t);
    let best = ranked[0].0;
    roster::set_plan(&mut w, w.people[best].player.id, Some(roster::Cut::Release)).unwrap();
    assert_eq!(w.front_office.owner(Duty::Cutdown), Owner::Staff, "a move took the duty back");
    let d = w.today;
    pro::cutdown(&mut w, d);
    assert_ne!(w.people[best].pro.as_ref().and_then(|c| c.team), Some(t), "your cut stayed");
    assert_eq!(roster::count(&w, t, Status::Roster) + roster::count(&w, t, Status::Reserve), 40);
}

#[test]
fn injured_reserve_is_your_call_on_the_leagues_rules() {
    // Minor bug 73: your player goes on injured reserve only when you put
    // him there; he sits the minimum, and coming back uses a return.
    let (mut w, t) = nfl_world();
    assert!(fo::yours(&w, t, Duty::InjuredReserve), "injured reserve starts yours");
    let i = *w.clubs[t].roster.iter().find(|&&i| w.people[i].pro.as_ref().unwrap().status == Status::Roster).unwrap();
    let id = w.people[i].player.id;
    assert!(fo::reserve::place(&mut w, id).is_err(), "a healthy player went on reserve");
    w.people[i].pro.as_mut().unwrap().out_weeks = 2;
    fo::reserve::place(&mut w, id).unwrap();
    let c = w.people[i].pro.as_ref().unwrap();
    assert_eq!(c.status, Status::Reserve);
    assert_eq!(c.out_weeks, pro::games::ir_minimum(w.year), "he must sit the list's minimum");
    assert!(fo::reserve::activate(&mut w, id).is_err(), "back before he healed");
    w.people[i].pro.as_mut().unwrap().out_weeks = 0;
    // Room on the 53 for him.
    while roster::count(&w, t, Status::Roster) >= w.rules.nfl_roster as usize {
        let j = *w.clubs[t]
            .roster
            .iter()
            .find(|&&j| j != i && w.people[j].pro.as_ref().unwrap().status == Status::Roster)
            .unwrap();
        w.people[j].pro.as_mut().unwrap().status = Status::PracticeSquad;
    }
    let used = w.clubs[t].ir_used;
    fo::reserve::activate(&mut w, id).unwrap();
    assert_eq!(w.people[i].pro.as_ref().unwrap().status, Status::Roster);
    assert_eq!(w.clubs[t].ir_used, used + 1, "a return was used");
}
