//! Capital lane C1: the donors (`world/capital/`).

use crate::calendar::Date;
use crate::world::World;
use crate::world::capital::donors::{self, Money};
use crate::world::capital::{expros, pledges, screen};
use crate::world::coach::Employer;
use crate::world::data::Division;
use crate::world::folk::Team;
use crate::world::folk::cast::{self, Role};

/// The tiny college seat with its donors built.
fn world() -> (World, usize, u32) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let you = w.career.as_ref().unwrap().coach as u32;
    if w.capital.year == 0 {
        let d = w.today;
        crate::world::capital::day(&mut w, d);
    }
    (w, prog, you)
}

#[test]
fn every_college_keeps_its_donors_with_the_boosters_on_top() {
    let (w, _, _) = world();
    let mut p4 = Vec::new();
    for p in 0..w.programs.len() {
        let n = w.capital.donors.iter().filter(|d| d.alive() && d.home == p as u32 && d.pro.is_none()).count();
        let prog = &w.programs[p];
        if prog.division == Division::JUCO {
            assert_eq!(n, 0, "{}", prog.real_name);
            continue;
        }
        // The target plus the boosters' slots the talks know.
        let t = donors::target(&w, p);
        assert!(n >= t && n <= t + 5, "{} has {n}, target {t}", prog.real_name);
        if prog.division == Division::FBS && prog.power_conference(w.year) {
            assert!((22..=30).contains(&t));
            p4.push(n);
        }
        // Each booster the talks know is a donor of the same name.
        for c in cast::all(&w, Team::College(p), Role::Booster) {
            let id = donors::of_booster(&w, c.r, c.term).expect("the booster is a donor");
            assert_eq!(w.capital.donor(id).unwrap().name, c.name);
        }
    }
    assert!(!p4.is_empty());
    let all: Vec<_> = w.capital.donors.iter().filter(|d| d.alive()).collect();
    let new = all.iter().filter(|d| d.money == Money::New).count() as f64 / all.len() as f64;
    let circled = |m: Money| {
        let v: Vec<_> = all.iter().filter(|d| d.money == m && d.pro.is_none()).collect();
        v.iter().filter(|d| d.in_circle().is_some()).count() as f64 / v.len() as f64
    };
    let two = all.iter().filter(|d| d.also.is_some()).count();
    let stretched = all.iter().filter(|d| d.stretched()).count() as f64 / all.len() as f64;
    println!(
        "{} donors, P4 mean {:.1}, new money {:.2}, in a circle: old {:.2} new {:.2}, two schools {two}, stretched {stretched:.2}, circles {}",
        all.len(),
        p4.iter().sum::<usize>() as f64 / p4.len() as f64,
        new,
        circled(Money::Old),
        circled(Money::New),
        w.capital.circles.len()
    );
    assert!((0.3..0.7).contains(&new));
    assert!(circled(Money::Old) > circled(Money::New));
    assert!(two > 0);
    assert!((0.05..0.25).contains(&stretched));
    // Every circle member's circle is one of his school's.
    for d in &all {
        if let Some(c) = d.in_circle() {
            assert_eq!(w.capital.circle(c).unwrap().home, d.home);
        }
    }
    // A second school is in his state, and courts him too.
    let d = all.iter().find(|d| d.also.is_some()).unwrap();
    let q = d.also.unwrap() as usize;
    assert_eq!(w.programs[q].state, w.programs[d.home as usize].state);
    assert!(donors::of(&w, q).iter().any(|&k| w.capital.donors[k].id == d.id));
    let s: f32 = d.schools().iter().map(|x| x.1).sum();
    assert!((s - 1.0).abs() < 1e-5);
}

#[test]
fn the_wealth_read_is_a_range_of_words_and_the_stretched_read_rich() {
    let (w, prog, _) = world();
    let mine = donors::of(&w, prog);
    for &k in &mine {
        let d = &w.capital.donors[k];
        let r = screen::read(&w, prog, d, 0);
        assert!(r.lo <= r.hi);
        assert!(!r.words.contains('$') && !r.words.chars().any(|c| c.is_ascii_digit()), "{}", r.words);
        // Meetings narrow it.
        let r4 = screen::read(&w, prog, d, 4);
        assert!(r4.hi - r4.lo <= r.hi - r.lo);
    }
    // Across every donor, the stretched read above what they can give.
    let band = |x: f64| screen::BANDS.iter().rposition(|b| x >= b.0).unwrap_or(0) as f64;
    let (mut over, mut n) = (0.0, 0.0);
    for d in w.capital.donors.iter().filter(|d| d.stretched()) {
        let r = screen::read(&w, d.home as usize, d, 0);
        over += (r.lo + r.hi) as f64 / 2.0 - band(d.capacity / donors::dollars(w.year));
        n += 1.0;
    }
    println!("stretched donors read {:.2} bands above their real capacity ({n})", over / n);
    assert!(n > 0.0 && over / n > 0.4);
    assert_eq!(screen::words(2, 4), "Could name a room, maybe fund a whole building.");
}

#[test]
fn a_broken_word_spreads_to_old_money_in_the_circle() {
    let (mut w, _, you) = world();
    // An old-money circle with old and new money in it.
    let c = w.capital.circles.iter().find(|c| c.old).map(|c| c.id).unwrap();
    let ids: Vec<u32> = w.capital.donors.iter().filter(|d| d.in_circle() == Some(c)).map(|d| d.id).collect();
    assert!(ids.len() >= 2);
    let home = w.capital.donor(ids[0]).unwrap().home;
    let (old, new) = {
        let mut old = None;
        let mut new = None;
        for d in w.capital.donors.iter_mut().filter(|d| d.home == home && d.alive()) {
            if d.id == ids[0] {
                continue;
            }
            if old.is_none() {
                d.money = Money::Old;
                d.circle = Some(c);
                old = Some(d.id);
            } else if new.is_none() {
                d.money = Money::New;
                d.circle = Some(c);
                new = Some(d.id);
            }
        }
        (old.unwrap(), new.unwrap())
    };
    let outside = w.capital.donors.iter().find(|d| d.alive() && d.in_circle() != Some(c)).unwrap().id;
    donors::word(&mut w, ids[0], you, false);
    let warm = |w: &World, id: u32| w.capital.donor(id).unwrap().warmth_to(you);
    assert!((warm(&w, ids[0]) - (donors::STRANGER + donors::BROKEN_SELF)).abs() < 1e-6);
    assert!((warm(&w, old) - (donors::STRANGER + donors::HEARD_OLD)).abs() < 1e-6);
    assert!((warm(&w, new) - (donors::STRANGER + donors::HEARD_NEW)).abs() < 1e-6);
    assert_eq!(warm(&w, outside), donors::STRANGER);
    assert_eq!(w.capital.donor(old).unwrap().bond(you).unwrap().heard, 1);
    // A kept word is his own.
    donors::word(&mut w, old, you, true);
    assert_eq!(w.capital.donor(old).unwrap().bond(you).unwrap().kept, 1);
    assert!(warm(&w, old) > donors::STRANGER + donors::HEARD_OLD);
}

#[test]
fn a_booster_promise_reaches_his_donor() {
    let (mut w, prog, you) = world();
    let c = cast::all(&w, Team::College(prog), Role::Booster).remove(0);
    donors::booster_word(&mut w, c.r, c.term, false);
    let id = donors::of_booster(&w, c.r, c.term).unwrap();
    assert_eq!(w.capital.donor(id).unwrap().bond(you).unwrap().broken, 1);
}

#[test]
fn a_pledge_tied_to_the_coach_is_pulled_when_he_leaves() {
    let (mut w, prog, _) = world();
    let head = w.programs[prog].head_coach.expect("a head coach") as u32;
    let mine = donors::of(&w, prog);
    let warm = w.capital.donors[mine[0]].id;
    let cool = mine[1..].iter().map(|&k| &w.capital.donors[k]).find(|d| d.bond(head).is_none()).unwrap().id;
    w.capital.donors[mine[0]].bond_mut(head).warmth = 1.0;
    let today = w.today;
    let tied = pledges::make(&mut w, warm, prog as u32, 5.0e6, 5, Some(head), Some(1), today);
    let loose = pledges::make(&mut w, cool, prog as u32, 2.0e6, 4, Some(head), None, today);
    let get = |w: &World, id: u32| w.capital.pledges.iter().find(|p| p.id == id).unwrap().clone();
    assert_eq!(get(&w, tied).tied, Some(head));
    assert_eq!(get(&w, loose).tied, None, "a stranger's pledge is the school's");
    // He stays: nothing moves.
    pledges::day(&mut w, today);
    assert!(get(&w, tied).open());
    // He takes another college job.
    let other = (0..w.programs.len()).find(|&q| q != prog && w.programs[q].division == Division::FBS).unwrap();
    w.coaches[head as usize].job.as_mut().unwrap().employer = Employer::College(other);
    let next = today.add(1);
    pledges::day(&mut w, next);
    assert_eq!(get(&w, tied).pulled, Some(next));
    assert_eq!(get(&w, tied).left(), 0.0);
    assert!(get(&w, loose).open());
    let d = w.capital.donor(warm).unwrap();
    println!("followed: {:?} (pull {:.2}, affinity {:.2})", d.also, d.also_pull, d.affinity);
    if d.also == Some(other as u32) {
        assert!(d.also_pull >= pledges::FOLLOW_PULL);
    }
    // Followers, over many donors at full warmth: near the top chance.
    let ids: Vec<u32> =
        w.capital.donors.iter().filter(|d| d.alive() && d.home != other as u32).map(|d| d.id).take(200).collect();
    w.coaches[head as usize].job.as_mut().unwrap().employer = Employer::College(prog);
    for &id in &ids {
        let home = w.capital.donor(id).unwrap().home;
        w.capital.donor_mut(id).unwrap().bond_mut(head).warmth = 1.0;
        w.capital.donor_mut(id).unwrap().also = None;
        pledges::make(&mut w, id, home, 1.0e6, 3, Some(head), None, today);
    }
    w.coaches[head as usize].job.as_mut().unwrap().employer = Employer::College(other);
    pledges::day(&mut w, next.add(1));
    let followed = ids.iter().filter(|&&id| w.capital.donor(id).unwrap().also == Some(other as u32)).count();
    println!("{followed} of {} followed at full warmth", ids.len());
    assert!(followed > ids.len() / 5 && followed < ids.len() * 4 / 5);
}

#[test]
fn an_ex_pro_joins_his_school_at_his_second_contract() {
    let (mut w, _, _) = world();
    let year = w.year;
    // Every pro in the tiny world gets a big extension.
    let pros: Vec<usize> = (0..w.people.len())
        .filter(|&i| {
            w.people[i].stage == crate::world::Stage::Pro
                && w.people[i].pro.as_ref().is_some_and(|p| p.contract.is_some())
        })
        .collect();
    assert!(!pros.is_empty());
    let before = w.capital.donors.iter().filter(|d| d.pro.is_some()).count();
    w.capital.pros.clear();
    w.capital.donors.retain(|d| d.pro.is_none());
    // A rookie deal changes nothing.
    for &i in &pros {
        let c = w.people[i].pro.as_mut().unwrap().contract.as_mut().unwrap();
        c.kind = crate::world::pro::money::Kind::Rookie;
    }
    expros::year(&mut w);
    assert_eq!(w.capital.donors.iter().filter(|d| d.pro.is_some()).count(), 0);
    w.capital.pros.clear();
    for &i in &pros {
        let c = w.people[i].pro.as_mut().unwrap().contract.as_mut().unwrap();
        c.kind = crate::world::pro::money::Kind::Extension;
        c.signing_bonus = 40.0e6;
    }
    expros::year(&mut w);
    let joined: Vec<_> = w.capital.donors.iter().filter(|d| d.pro.is_some()).collect();
    println!(
        "{} of {} pros joined (before: {before}); first: {:?}",
        joined.len(),
        pros.len(),
        joined.first().map(|d| (&d.name, &d.business))
    );
    assert!(!joined.is_empty() && joined.len() < pros.len());
    for d in &joined {
        assert_eq!(d.money, Money::New);
        assert!(d.capacity > 0.0 && d.age(year) < 45);
        assert!(d.bonds.iter().all(|b| b.coached && b.warmth > donors::STRANGER));
    }
    // One look a pro: a second pass adds nobody.
    let n = joined.len();
    expros::year(&mut w);
    assert_eq!(w.capital.donors.iter().filter(|d| d.pro.is_some()).count(), n);
}

#[test]
fn donors_age_die_leave_estates_and_are_replaced() {
    let (mut w, _, _) = world();
    let start = w.capital.donors.iter().filter(|d| d.alive()).count();
    for _ in 0..10 {
        w.year += 1;
        crate::world::capital::donors::year(&mut w);
        w.capital.year = w.year;
    }
    let alive = w.capital.donors.iter().filter(|d| d.alive()).count();
    let dead = w.capital.donors.iter().filter(|d| d.died).count();
    println!(
        "{start} donors, ten years on {alive} on the list, {dead} recent deaths kept, {} estates",
        w.capital.estates.len()
    );
    assert!(!w.capital.estates.is_empty());
    assert!(w.capital.estates.iter().all(|e| e.amount > 0.0));
    assert!(alive as f64 > start as f64 * 0.9 && (alive as f64) < start as f64 * 1.1);
    assert!(w.capital.donors.windows(2).all(|x| x[0].id < x[1].id));
}

#[test]
fn the_store_saves_and_an_old_file_loads_empty() {
    let (w, _, _) = world();
    let bytes = rmp_serde::to_vec_named(&w.capital).unwrap();
    let back: crate::world::capital::Capital = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.donors.len(), w.capital.donors.len());
    let old: crate::world::capital::Capital = serde_json::from_str("{}").unwrap();
    assert!(old.donors.is_empty() && old.year == 0);
    println!("capital store: {} donors, {} bytes", w.capital.donors.len(), bytes.len());
}

/// The world plays the same with the donors: they only write their own
/// store.
/// The people store's parts that are the seat's own (lane C4's donor
/// scenes and names calls write only these).
const SEAT_FOLK: &[&str] = &["mine", "scenes", "next_scene", "log", "heard", "shown", "said_on"];

#[test]
fn the_world_plays_the_same_with_the_donors() {
    let fp = |w: &World| -> Vec<serde_json::Value> {
        vec![
            serde_json::to_value(&w.people).unwrap(),
            serde_json::to_value(&w.programs).unwrap(),
            serde_json::to_value(&w.coaches).unwrap(),
            // Lane C4's donor scenes are the seat's own: your scenes, your
            // log, what you said and the AD's trust in you. The rest of the
            // people store must not move.
            {
                let mut f = serde_json::to_value(&w.folk).unwrap();
                for k in SEAT_FOLK {
                    f.as_object_mut().unwrap().remove(*k);
                }
                f
            },
            serde_json::to_value(&w.finance).unwrap(),
        ]
    };
    let mut on = super::legacy::seat("college");
    let mut off = super::legacy::seat("college");
    off.capital.off = true;
    let kept = off.capital.donors.len();
    // Through the money year's day (1 July), where the donors' year runs.
    let end = Date::ymd(on.year, 7, 3).max(on.today.add(3));
    while on.today < end {
        on.advance_day();
        off.advance_day();
    }
    assert!(!on.capital.donors.is_empty() && on.capital.year == on.year);
    assert_eq!(off.capital.donors.len(), kept);
    let names = ["people", "programs", "coaches", "folk", "finance"];
    let (a, b) = (fp(&on), fp(&off));
    let mut moved: Vec<String> = (0..a.len()).filter(|&k| a[k] != b[k]).map(|k| names[k].to_string()).collect();
    if let (Some(x), Some(y)) = (a[3].as_object(), b[3].as_object()) {
        moved.extend(x.keys().filter(|k| x.get(*k) != y.get(*k)).map(|k| format!("folk.{k}")));
    }
    assert!(moved.is_empty(), "the world moved with the donors: {moved:?}");
}

/// Two schools' donors drawn with the same year and place are their own
/// men (the program is in the seed).
#[test]
fn each_school_draws_its_own_donors() {
    let (w, _, _) = world();
    let a = donors::draw_donor(&w, 0, w.year, w.year, 3);
    let b = donors::draw_donor(&w, 1, w.year, w.year, 3);
    assert!(a.name != b.name || a.capacity != b.capacity);
    let same = (0..200)
        .filter(|&n| {
            let x = donors::draw_donor(&w, 0, w.year, w.year, n);
            let y = donors::draw_donor(&w, 1, w.year, w.year, n);
            x.capacity == y.capacity
        })
        .count();
    assert_eq!(same, 0);
}
