//! Waiver claims: a club at its roster limit makes room for a claim only
//! with a release that frees cap room, never one whose dead money outweighs
//! what it saves.

use crate::positions::Position;
use crate::world::pro::{self, DeadWhy, Status, market, money};
use crate::world::{World, WorldOptions};

#[test]
fn a_full_club_never_cuts_a_big_contract_to_make_room_for_a_claim() {
    let mut w = World::empty(WorldOptions { scale: 0.001, history_years: 0, start_year: 2037, ..Default::default() });
    let (year, d) = (w.year, w.today);
    let june1 = pro::after_june1(&w, d);
    // The first club in the waiver order gets the first look.
    let t = market::waiver_order(&w, d)[0];
    let pos = Position::WR;
    let receivers = |w: &World| -> Vec<usize> {
        w.clubs[t]
            .roster
            .iter()
            .copied()
            .filter(|&j| {
                w.people[j].player.position == pos
                    && w.people[j].pro.as_ref().is_some_and(|c| c.status == Status::Roster && c.out_weeks == 0)
            })
            .collect()
    };
    let mine = receivers(&w);
    assert!(mine.len() >= 2, "the club has {} receivers", mine.len());
    // His worst receiver five months into a long extension: most of it
    // signing bonus, so a release now would cost far more than it saves.
    let worst = *mine.iter().min_by(|&&a, &&b| pro::grade(&w, t, a).total_cmp(&pro::grade(&w, t, b))).unwrap();
    let deal = money::build(
        t,
        money::Kind::Veteran,
        year,
        &money::Terms { bonus_share: 0.6, ..money::Terms::flat(5, 30.0e6) },
    );
    assert!(
        deal.cut_savings(year, d, june1) < -20.0e6,
        "the deal's release frees {}",
        deal.cut_savings(year, d, june1)
    );
    w.people[worst].pro.as_mut().unwrap().contract = Some(deal);
    // Room on the cap for any claim, and a roster at its limit.
    w.clubs[t].carryover += 400.0e6;
    let full = w.clubs[t].roster.len() as u32;
    w.rules.nfl_offseason = full;
    w.rules.nfl_roster = full;
    // The league's best receiver from elsewhere, young enough for waivers.
    let other = (0..w.people.len())
        .filter(|&i| {
            w.people[i].player.position == pos
                && w.people[i].pro.as_ref().is_some_and(|c| {
                    c.status == Status::Roster && c.team.is_some_and(|x| x != t) && c.contract.is_some()
                })
        })
        .max_by(|&a, &b| pro::grade(&w, t, a).total_cmp(&pro::grade(&w, t, b)))
        .unwrap();
    assert!(pro::grade(&w, t, other) >= pro::grade(&w, t, worst) + 2.0, "the claim is an upgrade on him");
    let c = w.people[other].pro.as_mut().unwrap();
    c.credited = 1;
    let k = c.contract.as_mut().unwrap();
    *k = money::build(k.team, money::Kind::Minimum, year, &money::Terms::flat(1, money::minimum(year, 1)));
    pro::release_with(&mut w, other, d, "a test release", false, true, DeadWhy::Released);
    assert!(w.people[other].pro.as_ref().unwrap().on_waivers);
    // What each receiver's release would free today.
    let frees: Vec<(usize, f64)> = mine
        .iter()
        .map(|&j| {
            let k = w.people[j].pro.as_ref().unwrap().contract.as_ref();
            (j, k.map_or(0.0, |k| if k.team == t { k.cut_savings(year, d, june1) } else { 0.0 }))
        })
        .collect();
    market::claims(&mut w, d);
    // He keeps his job: the club claims only with a cheaper release, or not at all.
    assert!(w.clubs[t].roster.contains(&worst), "the club cut its $150M receiver for a waiver claim");
    let id = w.people[worst].player.id;
    assert!(!w.clubs[t].dead.iter().any(|x| x.player == id), "dead money for the receiver it kept");
    // Whoever it let go freed cap room: his dead money no more than his cap number.
    let gone: Vec<(usize, f64)> = frees.into_iter().filter(|(j, _)| !w.clubs[t].roster.contains(j)).collect();
    if w.people[other].pro.as_ref().unwrap().team == Some(t) {
        assert_eq!(gone.len(), 1, "one receiver made room");
    }
    assert!(gone.iter().all(|x| x.1 >= 0.0), "a release that cost cap room: {gone:?}");
}
