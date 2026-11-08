//! Placing a player and the favours it earns (minor bugs 323-327 and 331,
//! thread "Fix placement and contact bugs", 2026-11-07): the NFL cutdown
//! swap is a trade the other GM must take, a college or NFL coach's word to
//! a college staff lasts to the offers, a placed player's bond holds until
//! he decides in the portal, a favour balance never passes five either way,
//! your own player's no-trade clause holds, and an ask he can't do costs no
//! favour.

use super::legacy::seat;
use crate::calendar::Weekday;
use crate::player::PlayingTime;
use crate::world::clock::PortalWindow;
use crate::world::coach::Employer;
use crate::world::data::Division;
use crate::world::folk::backchannel;
use crate::world::folk::network::{self, Contact, How, MAX_BALANCE, Why};
use crate::world::front_office::deals::{self, Proposal, Side};
use crate::world::pro::{self, Status};
use crate::world::{Stage, World, recruiting};

/// Puts a coach in your book at this warmth, owing nothing either way.
fn know(w: &mut World, c: usize, warmth: f32) -> usize {
    if let Some(k) = w.folk.network.contacts.iter().position(|x| x.coach_id() == Some(c)) {
        let x = &mut w.folk.network.contacts[k];
        x.warmth = warmth;
        x.balance = 0;
        x.sent = 0;
        return k;
    }
    w.folk.network.contacts.push(Contact {
        coach: c,
        warmth,
        met: w.today,
        how: "a test".into(),
        last: None,
        recent: Vec::new(),
        balance: 0,
        other: None,
        sent: 0,
        record: (0, 0),
    });
    w.folk.network.contacts.len() - 1
}

/// Your club's healthy players with a deal that can be traded, by what
/// each is worth to club `o`, lowest first.
fn tradable(w: &World, t: usize, o: usize) -> Vec<(usize, f64)> {
    let year = w.year;
    let mut v: Vec<(usize, f64)> = w.clubs[t]
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            let c = w.people[i].pro.as_ref().unwrap();
            c.team == Some(t)
                && c.status != Status::Reserve
                && c.out_weeks == 0
                && c.contract.as_ref().is_some_and(|k| k.team == t && k.last_year() >= year && !k.no_trade)
        })
        .map(|i| (i, deals::player_points(w, o, i)))
        .collect();
    v.sort_by(|a, b| a.1.total_cmp(&b.1));
    v
}

#[test]
fn a_cutdown_swap_is_a_trade_the_other_gm_must_take() {
    let mut w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    w.today = w.dates.cutdown.date.add(-3);
    assert!(deals::window_open(&w));
    // A friend on the staff of the club with the most cap room.
    let year = w.year;
    let (c, o) = (0..w.coaches.len())
        .filter_map(|c| match w.coaches[c].job.map(|j| j.employer) {
            Some(Employer::Nfl(o)) if o != t && crate::world::franchise::active_in(&w, o, year) => Some((c, o)),
            _ => None,
        })
        .max_by(|a, b| pro::space(&w, a.1, year).total_cmp(&pro::space(&w, b.1, year)))
        .expect("a coach on another club's staff");
    let room = pro::space(&w, o, year);
    let k = know(&mut w, c, 0.6);
    let squad = tradable(&w, t, o);
    let picks = |w: &World| (deals::picks_of(w, t), deals::picks_of(w, o));
    let before = picks(&w);
    let ledger = w.folk.network.ledger.len();
    // The worst deal on your roster: his GM says no. Before, any player and
    // your latest pick always bought his pick a round higher, and a favour.
    let (worst, value) = squad[0];
    let id = w.people[worst].player.id;
    let e = backchannel::place_player(&mut w, id, c).expect_err("the swap went through without the GM's yes");
    assert!(!e.contains("cutdown-day") && !e.contains("isn't on your roster"), "{e}");
    assert!(value < 40.0, "{value}");
    assert_eq!(w.people[worst].pro.as_ref().unwrap().team, Some(t));
    assert_eq!(picks(&w), before, "picks moved on a no");
    let x = &w.folk.network.contacts[k];
    assert_eq!((x.balance, x.sent), (0, 0), "a favour for a deal that didn't happen");
    assert_eq!(w.folk.network.ledger.len(), ledger);
    // The most valuable one whose pay fits under his cap: he takes it, the
    // player and the picks move, and he owes you one.
    let pay = |w: &World, i: usize| w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap().cap_hit(year);
    let (best, _) = *squad.iter().rev().find(|&&(i, _)| pay(&w, i) < room - 1.0e6).expect("a player his cap can take");
    let id = w.people[best].player.id;
    let text = backchannel::place_player(&mut w, id, c).unwrap();
    assert!(text.starts_with("Traded to the"), "{text}");
    assert_eq!(w.people[best].pro.as_ref().unwrap().team, Some(o));
    assert_ne!(picks(&w), before);
    let x = &w.folk.network.contacts[k];
    assert_eq!((x.balance, x.sent), (1, 1));
}

#[test]
fn your_own_no_trade_player_waives_only_for_a_better_club() {
    let mut w = seat("nfl");
    let t = w.seat.and_then(|s| s.nfl_team).unwrap();
    w.today = w.dates.league_year.date.add(2);
    // Clubs with room for a cheap deal, so only the clause can say no.
    let year = w.year;
    let others: Vec<usize> = (0..w.clubs.len())
        .filter(|&o| o != t && crate::world::franchise::active_in(&w, o, year) && pro::space(&w, o, year) > 5.0e6)
        .collect();
    let by = |w: &World, o: usize| w.nfl[o].strength();
    let weak = *others.iter().min_by(|&&a, &&b| by(&w, a).total_cmp(&by(&w, b))).unwrap();
    let strong = *others.iter().max_by(|&&a, &&b| by(&w, a).total_cmp(&by(&w, b))).unwrap();
    let mine = by(&w, t);
    assert!(by(&w, weak) < mine + 2.0, "no club below yours + 2 ({mine})");
    // Your cheapest player, with a no-trade clause.
    let pay = |w: &World, i: usize| w.people[i].pro.as_ref().unwrap().contract.as_ref().unwrap().cap_hit(year);
    let i = tradable(&w, t, weak).into_iter().map(|x| x.0).min_by(|&a, &b| pay(&w, a).total_cmp(&pay(&w, b))).unwrap();
    let id = w.people[i].player.id;
    w.people[i].pro.as_mut().unwrap().contract.as_mut().unwrap().no_trade = true;
    let last = |w: &World, o: usize| {
        let year = deals::picks_of(w, o).iter().map(|p| p.year).min().unwrap();
        deals::picks_of(w, o).into_iter().filter(|p| p.year == year && !p.comp).max_by_key(|p| p.round).unwrap()
    };
    let ask = |w: &World, o: usize| {
        let p = Proposal {
            other: o,
            give: Side { players: vec![id], picks: Vec::new() },
            get: Side { players: Vec::new(), picks: vec![last(w, o)] },
            conditional: None,
        };
        match deals::evaluate(w, &p) {
            Ok(a) => a.text,
            Err(e) => e,
        }
    };
    // Before, his clause was never read when you sent him.
    let no = ask(&w, weak);
    assert!(no.contains("no-trade clause"), "{no}");
    if by(&w, strong) >= mine + 2.0 {
        let a = ask(&w, strong);
        assert!(!a.contains("no-trade clause"), "{a}");
    }
    // Shopping him skips the clubs he won't go to.
    let offers = deals::shop(&mut w, id).unwrap();
    assert!(offers.iter().all(|x| by(&w, x.0) >= mine + 2.0), "{offers:?}");
    // A call you answer yes keeps the clause too.
    w.front_office.trades.calls.push(deals::Call {
        date: w.today,
        until: w.today.add(2),
        buyer: weak,
        player: id,
        picks: vec![last(&w, weak)],
        answered: false,
    });
    let k = w.front_office.trades.calls.len() - 1;
    let e = deals::answer_call(&mut w, k, true).expect_err("traded past his clause on a call");
    assert!(e.contains("no-trade clause"), "{e}");
    assert_eq!(w.people[i].pro.as_ref().unwrap().team, Some(t));
}

#[test]
fn a_word_to_a_college_staff_reaches_every_signing_stage() {
    for level in ["college", "nfl"] {
        let mut w = seat(level);
        w.add_class_aged(17);
        let mine = w.seat.and_then(|s| s.program);
        let c = w
            .programs
            .iter()
            .enumerate()
            .filter(|&(q, _)| Some(q) != mine)
            .find_map(|(_, p)| p.head_coach)
            .expect("another program's head coach");
        let k = know(&mut w, c, 1.0);
        w.folk.network.contacts[k].balance = MAX_BALANCE;
        let kid = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).expect("a high school kid");
        let id = w.people[kid].player.id;
        let seen = w.people[kid].visibility;
        let start = w.today;
        let mut said = false;
        for n in 0..12 {
            w.today = start.add(35 * n);
            if let Ok(text) = network::reach_out(&mut w, c, How::Call, Why::Word, Some(id))
                && text.contains("have his staff look")
            {
                said = true;
                break;
            }
        }
        assert!(said, "{level}: he never said yes");
        // Before, the word added 0.12 to his visibility, which the next
        // signing stage rebuilt from scratch: now it is a call to that
        // staff, which every stage reads.
        assert_eq!(w.people[kid].visibility, seen, "{level}");
        let bump = crate::world::scouting::hs::promos(&w).get(&id).copied().unwrap_or(0.0);
        assert!(bump > 0.0, "{level}: {bump}");
    }
}

#[test]
fn a_placed_players_bond_lasts_until_he_decides() {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.cal.lean = 0.5;
    }
    w.programs[p].roster.push(i);
    // A friend heading a smaller program.
    let mine = w.programs[p].division as usize;
    let (q, c) = w
        .programs
        .iter()
        .enumerate()
        .filter(|(_, x)| x.division as usize > mine && x.division != Division::JUCO)
        .find_map(|(q, x)| x.head_coach.map(|c| (q, c)))
        .expect("a smaller program's head coach");
    know(&mut w, c, 0.6);
    let id = w.people[i].player.id;
    backchannel::place_player(&mut w, id, c).unwrap();
    let close = w.today.add(15);
    recruiting::portal_open(&mut w, PortalWindow::Winter, close);
    let e = w.people[i].cal.portal.expect("he enters the window with your blessing");
    // Every Monday before his day keeps the friend's interest. Before, the
    // first Monday after he entered dropped it.
    let mut d = w.today;
    let mut mondays = 0;
    while d < e.decide_by {
        if d.weekday() == Weekday::Mon {
            w.today = d;
            backchannel::day(&mut w, d);
            mondays += 1;
        }
        d = d.add(1);
    }
    assert!(mondays > 0, "no Monday before {:?}", e.decide_by);
    assert!(backchannel::placed(&w, id), "dropped before he decided");
    assert!(backchannel::bonds(&w).contains_key(&(id, q)));
    // He decides: the placement is spent.
    w.today = e.decide_by;
    recruiting::portal_day(&mut w, true);
    assert!(w.people[i].cal.portal.is_none());
    assert!(!backchannel::placed(&w, id));
}

#[test]
fn a_favour_balance_holds_to_five_either_way() {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program);
    let c = w.programs.iter().enumerate().filter(|&(q, _)| Some(q) != mine).find_map(|(_, p)| p.head_coach).unwrap();
    let k = know(&mut w, c, 1.0);
    let x = &mut w.folk.network.contacts[k];
    x.shift(1);
    assert_eq!(x.balance, 1);
    for _ in 0..8 {
        x.shift(1);
    }
    assert_eq!(x.balance, MAX_BALANCE);
    for _ in 0..12 {
        x.shift(-1);
    }
    assert_eq!(x.balance, -MAX_BALANCE);
    // An ask he says yes to takes a favour, never past five you owe (the
    // call was the one path left without the limit).
    let start = w.today;
    let mut yes = 0;
    for n in 0..20 {
        w.today = start.add(35 * n);
        if let Ok(text) = network::reach_out(&mut w, c, How::Call, Why::Grapevine, None)
            && !text.contains("kept his cards close")
        {
            yes += 1;
        }
        assert!(w.folk.network.contacts[k].balance >= -MAX_BALANCE);
    }
    assert!(yes > 0, "he never said yes");
    assert_eq!(w.folk.network.contacts[k].balance, -MAX_BALANCE);
}

#[test]
fn an_ask_he_cant_do_costs_no_favour() {
    let mut w = seat("college");
    w.add_class_aged(17);
    // A high school coach has no next-level staff to look at a kid.
    let c = w.high_schools.iter().find_map(|h| h.coach).expect("a high school coach");
    let k = know(&mut w, c, 1.0);
    w.folk.network.contacts[k].balance = MAX_BALANCE;
    let kid = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let id = w.people[kid].player.id;
    let start = w.today;
    let mut asked = 0;
    for n in 0..6 {
        w.today = start.add(35 * n);
        if let Ok(text) = network::reach_out(&mut w, c, How::Call, Why::Word, Some(id)) {
            assert!(text.contains("not in a position to help"), "{text}");
            asked += 1;
        }
    }
    assert!(asked > 0);
    // Before, a yes still put "a look at your player" on the ledger.
    assert_eq!(w.folk.network.contacts[k].balance, MAX_BALANCE);
    assert!(!w.folk.network.ledger.iter().any(|f| f.coach == c && f.asked));
}
