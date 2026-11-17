//! Capital lane C3: the money and the drive (`world/capital/fever.rs`,
//! `gifts.rs`, `drive.rs`; `docs/design/capital.md`, CP7-CP12, CP14-CP16,
//! CP24, CP27).

use crate::calendar::Date;
use crate::world::World;
use crate::world::capital::drive::{self, BREAK, Fix, Size, Stage};
use crate::world::capital::fever::{self, Moment, What};
use crate::world::capital::gifts::{self, Answer, Side};
use crate::world::capital::{self, donors};
use crate::world::data::Division;
use crate::world::finance::{Owner, ProjectKind};

/// The tiny college seat with its donors built, and an AI FBS program.
fn world() -> (World, usize, usize) {
    let mut w = super::legacy::seat("college");
    if w.capital.year == 0 {
        let d = w.today;
        capital::day(&mut w, d);
    }
    let mine = w.seat.and_then(|s| s.program).unwrap();
    let ai = (0..w.programs.len())
        .find(|&q| q != mine && w.programs[q].division == Division::FBS && w.programs[q].power_conference(w.year))
        .unwrap();
    (w, mine, ai)
}

/// Every donor of a program rich, close to the school and building-minded.
fn rich(w: &mut World, p: usize) {
    for k in donors::of(w, p) {
        let d = &mut w.capital.donors[k];
        d.capacity = 30.0e6;
        d.affinity = 0.9;
        d.nil_lean = 1.0;
        d.collects = 0.0;
    }
}

/// The first of each month from `from`, `n` months, the drives' step run.
fn months(w: &mut World, n: i32) {
    for _ in 0..n {
        let (y, m, _) = w.today.parts();
        let next = if m == 12 { Date::ymd(y + 1, 1, 1) } else { Date::ymd(y, m + 1, 1) };
        w.today = next;
        drive::month(w, next);
    }
}

/// CP7 and CP8: a top-10 pick starts the fever like a conference title,
/// half a national title, the Heisman between; a moment fades over two
/// years, fast at first, and a run stacks.
#[test]
fn the_fever_starts_on_big_moments_stacks_and_fades() {
    assert_eq!(What::TopTenPick.strength(), What::ConferenceTitle.strength());
    assert_eq!(What::ConferenceTitle.strength() * 2.0, What::NationalTitle.strength());
    assert!(What::Heisman.strength() > What::ConferenceTitle.strength());
    assert!(What::Heisman.strength() < What::NationalTitle.strength());
    assert!(What::Playoff.strength() > What::BowlWin.strength());
    assert!(What::BowlWin.strength() > What::RivalryWin.strength());
    let m = |date: Date, what| Moment { program: 0, date, what, who: None };
    // One good year: a conference title in December 2027, then a 5-7 year.
    let one = vec![m(Date::ymd(2027, 12, 6), What::ConferenceTitle)];
    let at = |v: &[Moment], d: Date| fever::fever_on(v, 0, d);
    let start = at(&one, Date::ymd(2027, 12, 6));
    let spring = at(&one, Date::ymd(2029, 4, 1));
    println!("one title: {start:.2} the day, {spring:.3} by the spring after next");
    assert!((start - 0.39).abs() < 0.01);
    assert!(spring < 0.06, "gone by the next spring");
    assert_eq!(at(&one, Date::ymd(2029, 12, 31)), 0.0);
    // A run: the title in 2027, the title and the playoff in 2028.
    let mut run = one.clone();
    run.push(m(Date::ymd(2028, 12, 5), What::ConferenceTitle));
    run.push(m(Date::ymd(2028, 12, 20), What::Playoff));
    let held = at(&run, Date::ymd(2029, 3, 1));
    println!("a run: {held:.2} in March 2029");
    assert!(held > 0.4 && held > 8.0 * spring);
    // Another program's moments are its own.
    assert_eq!(fever::fever_on(&run, 1, Date::ymd(2029, 3, 1)), 0.0);
    // It never passes the top.
    let lots: Vec<Moment> = (0..20).map(|_| m(Date::ymd(2028, 1, 1), What::NationalTitle)).collect();
    assert!(at(&lots, Date::ymd(2028, 1, 2)) < 1.0);
}

/// The fever lifts a donor's odds (CP7), a building with his room more than
/// one without, and a NIL-minded donor gives the building less (CP9).
#[test]
fn a_donor_gives_more_on_the_fever_for_his_room() {
    let (w, _, p) = world();
    let k = donors::of(&w, p)[0];
    let mut d = w.capital.donors[k].clone();
    d.rooms = vec![1];
    d.nil_lean = 0.5;
    let weight = 1u16 << 1;
    let other = 1u16 << 9;
    let calm = gifts::odds(&w, p, &d, weight, None, 0.0);
    let hot = gifts::odds(&w, p, &d, weight, None, 0.8);
    let off = gifts::odds(&w, p, &d, other, None, 0.0);
    println!(
        "calm {:.2}/{:.2}, hot {:.2}/{:.2}, off-room {:.2}/{:.2}",
        calm.chance, calm.share, hot.chance, hot.share, off.chance, off.share
    );
    assert!(hot.chance > calm.chance && hot.share > calm.share);
    assert!(off.chance < calm.chance && off.share < calm.share);
    d.nil_lean = -0.8;
    let nil = gifts::odds(&w, p, &d, weight, None, 0.0);
    assert!(nil.chance < calm.chance && nil.share < calm.share);
    // A warm coach in the room lifts them.
    let mut warm = d.clone();
    warm.bond_mut(7).warmth = 1.0;
    let with = gifts::odds(&w, p, &warm, weight, Some(7), 0.0);
    assert!(with.chance > nil.chance && with.share > nil.share);
}

/// A drive at an AI program runs its real stages: the study, the quiet
/// phase, the public launch, the groundbreaking and the close (CP12, CP27).
#[test]
fn an_ai_drive_runs_its_stages_and_breaks_ground() {
    let (mut w, _, p) = world();
    rich(&mut w, p);
    w.finance.projects.retain(|x| x.owner != Owner::College(p));
    w.today = Date::ymd(w.year, 7, 1);
    let id = drive::start(&mut w, p).expect("a drive starts");
    let get = |w: &World| w.capital.drives.iter().find(|d| d.id == id).unwrap().clone();
    assert_eq!(get(&w).stage, Stage::Study);
    assert!(drive::start(&mut w, p).is_none(), "one drive at a time");
    months(&mut w, 5);
    let x = get(&w);
    println!("study: advised {:?}, read {:.0}, goal {:.0}, leads {}", x.advised, x.expected, x.goal, x.lead.len());
    assert_eq!(x.stage, Stage::Quiet);
    assert_eq!(x.advised, Some(Size::Full), "rich donors carry the full complex");
    assert!(!x.lead.is_empty());
    let mut seen = vec![x.stage];
    for _ in 0..60 {
        months(&mut w, 1);
        let x = get(&w);
        if seen.last() != Some(&x.stage) {
            seen.push(x.stage);
        }
        if x.stage == Stage::Closed {
            break;
        }
    }
    let x = get(&w);
    let raised = drive::raised(&w.capital, &x);
    println!("stages {seen:?}; raised {raised:.0} of {:.0}; ground {:?}, opens {:?}", x.goal, x.ground, x.opens);
    assert_eq!(seen, vec![Stage::Quiet, Stage::Public, Stage::Closed]);
    assert!(x.public_on.is_some() && !x.early);
    assert!(x.ground.is_some() && raised >= BREAK * x.goal);
    let pr = w
        .finance
        .projects
        .iter()
        .find(|y| y.owner == Owner::College(p) && y.kind == ProjectKind::Facility && Some(y.started) == x.ground)
        .expect("the building is in the books");
    assert!(pr.pledged > 0.0 && pr.pledged <= pr.cost);
    assert_eq!(drive::rooms_of_project(&w, p, pr), x.rooms);
    // Lead gifts came through the AI head coach in the room.
    let coach = w.programs[p].head_coach.unwrap() as u32;
    let leads_met =
        x.lead.iter().filter(|&&l| w.capital.donor(l).and_then(|d| d.bond(coach)).is_some_and(|b| b.met > 0)).count();
    assert!(leads_met > 0, "the AI head coach met his lead donors");
}

/// A donor who leans to NIL and gives to the building takes it out of his
/// NIL money; pushed past his lean, all of it (CP9).
#[test]
fn a_building_gift_from_a_nil_donor_thins_the_collective() {
    let (mut w, _, p) = world();
    let k = donors::of(&w, p)[0];
    let id = w.capital.donors[k].id;
    w.capital.donors[k].nil_lean = -1.0;
    w.capital.donors[k].collects = 0.0;
    w.capital.donors[k].capacity = 10.0e6;
    let base = 50.0e6;
    w.programs[p].collective.base = base;
    gifts::give(&mut w, 0, p, id, 3.0e6, None, false, None);
    let years = w.capital.pledges.last().unwrap().years as f64;
    let once = base - w.programs[p].collective.base;
    assert!((once - 0.5 * 3.0e6 / years).abs() < 1.0, "dent {once}");
    w.programs[p].collective.base = base;
    gifts::give(&mut w, 0, p, id, 3.0e6, None, true, None);
    let pushed = base - w.programs[p].collective.base;
    assert!((pushed - 2.0 * once).abs() < 1.0);
    // A building-minded donor takes nothing from it.
    w.capital.donors[k].nil_lean = 1.0;
    w.programs[p].collective.base = base;
    gifts::give(&mut w, 0, p, id, 3.0e6, None, false, None);
    assert_eq!(w.programs[p].collective.base, base);
}

/// Your ask (CP9): a donor who leans the other way says he'd rather give to
/// the other side unless you push; one ask a stage; the head coach only.
#[test]
fn you_ask_donor_by_donor_for_the_building_or_nil() {
    let (mut w, mine, _) = world();
    w.finance.projects.retain(|x| x.owner != Owner::College(mine));
    w.today = Date::ymd(w.year, 7, 1);
    drive::start(&mut w, mine).expect("a drive");
    let first = w.capital.donors[0].id;
    assert!(gifts::ask(&mut w, mine, first, Side::Building, false).is_err(), "not during the study");
    let k = drive::open_drive(&w.capital, mine).unwrap();
    w.capital.drives[k].stage = Stage::Quiet;
    let list = donors::of(&w, mine);
    let a = w.capital.donors[list[0]].id;
    w.capital.donors[list[0]].nil_lean = -0.9;
    let (ans, words) = gifts::ask(&mut w, mine, a, Side::Building, false).unwrap();
    println!("{words}");
    assert_eq!(ans, Answer::Rather(Side::Nil));
    // The answer left the ask open: pushing is still possible, once.
    let (ans, words) = gifts::ask(&mut w, mine, a, Side::Building, true).unwrap();
    println!("pushed: {words}");
    assert!(matches!(ans, Answer::Pledged(_) | Answer::No));
    assert!(gifts::ask(&mut w, mine, a, Side::Building, true).is_err(), "one ask a stage");
    // You were in the room.
    let you = w.career.as_ref().unwrap().coach as u32;
    assert!(w.capital.donor(a).unwrap().bond(you).is_some_and(|b| b.met >= 2));
    // NIL from a NIL-minded donor, once a year.
    let before = w.programs[mine].collective.base;
    let (ans, _) = gifts::ask(&mut w, mine, a, Side::Nil, false).unwrap();
    match ans {
        Answer::Nil(x) => assert!((w.programs[mine].collective.base - before - x).abs() < 1.0),
        Answer::No => assert_eq!(w.programs[mine].collective.base, before),
        other => panic!("{other:?}"),
    }
    assert!(gifts::ask(&mut w, mine, a, Side::Nil, false).is_err());
    // Many pushes against the lean: some say yes, fewer than without a lean.
    let mut yes = 0;
    for &j in list.iter().skip(1) {
        w.capital.donors[j].nil_lean = -0.9;
        w.capital.donors[j].affinity = 0.9;
        let id = w.capital.donors[j].id;
        if let Ok((Answer::Pledged(_), _)) = gifts::ask(&mut w, mine, id, Side::Building, true) {
            yes += 1;
        }
    }
    println!("{yes} of {} pushed asks said yes", list.len() - 1);
}

/// A gift can carry a catch he keeps quiet (CP10): it rings a year to three
/// on; the AD answers at once for an AI program; yours waits for you, then
/// the AD answers.
#[test]
fn gifts_carry_catches_that_call_later() {
    let (mut w, mine, ai) = world();
    let today = w.today;
    for &p in &[mine, ai] {
        for k in donors::of(&w, p) {
            w.capital.donors[k].collects = 1.0;
        }
        for k in donors::of(&w, p) {
            let id = w.capital.donors[k].id;
            gifts::give(&mut w, 0, p, id, 1.0e6, None, false, None);
        }
    }
    let n = |w: &World, p: usize| w.capital.catches.iter().filter(|c| c.program == p as u32).count();
    println!("catches: yours {}, AI {}", n(&w, mine), n(&w, ai));
    assert!(n(&w, mine) > 0 && n(&w, ai) > 0);
    for c in w.capital.catches.iter() {
        let days = today.days_to(c.due);
        assert!((365..=1095).contains(&days), "due in {days} days");
    }
    // Every call comes due.
    let due = today.add(1100);
    w.today = due;
    gifts::catches(&mut w, due);
    assert!(w.capital.catches.iter().filter(|c| c.program == ai as u32).all(|c| c.answer.is_some()));
    let yours: Vec<u32> = w.capital.catches.iter().filter(|c| c.program == mine as u32).map(|c| c.id).collect();
    assert!(
        w.capital
            .catches
            .iter()
            .filter(|c| c.program == mine as u32)
            .all(|c| c.called == Some(due) && c.answer.is_none())
    );
    // You answer one; the AD answers the rest after thirty days.
    let words = gifts::answer_catch(&mut w, yours[0], false).unwrap();
    println!("{words}");
    let later = due.add(gifts::CATCH_DAYS);
    w.today = later;
    gifts::catches(&mut w, later);
    assert!(w.capital.catches.iter().filter(|c| c.program == mine as u32).all(|c| c.answer.is_some()));
    // The staff's read speaks in words.
    let d = w.capital.donors[donors::of(&w, mine)[0]].clone();
    assert_eq!(capital::screen::catch_read(&w, mine, &d, 4), "Likes to collect.");
}

/// Your drive (CP14-CP16, CP11): the office runs it until you take it over;
/// you set the goal, go public early, push the AD to borrow, and borrowing
/// costs the staff pool for years.
#[test]
fn your_drive_takes_your_calls() {
    let (mut w, mine, _) = world();
    w.finance.projects.retain(|x| x.owner != Owner::College(mine));
    w.today = Date::ymd(w.year, 7, 1);
    drive::start(&mut w, mine).expect("a drive");
    let k = drive::open_drive(&w.capital, mine).unwrap();
    assert!(!w.capital.drives[k].hands_on, "the development office runs it until you take it over");
    let words = drive::set_goal(&mut w, mine, Size::Full).unwrap();
    println!("{words}");
    assert!(w.capital.drives[k].set_by_you);
    months(&mut w, 5);
    assert_eq!(w.capital.drives[k].stage, Stage::Quiet);
    assert_eq!(w.capital.drives[k].size, Size::Full, "your goal stands");
    // Hands on: the lead donors wait for you.
    drive::hands_on(&mut w, mine, true).unwrap();
    months(&mut w, 2);
    let x = w.capital.drives[k].clone();
    assert!(x.lead.iter().all(|l| !x.asked.iter().any(|a| a.0 == *l)), "your leads wait for you");
    // Going public now is early.
    let words = drive::go_public(&mut w, mine).unwrap();
    println!("{words}");
    assert_eq!(w.capital.drives[k].stage, Stage::Public);
    assert!(
        w.capital.drives[k].early || drive::raised(&w.capital, &w.capital.drives[k]) >= 0.5 * w.capital.drives[k].goal
    );
    // Borrowing: half the goal in first.
    let goal = w.capital.drives[k].goal;
    let got = drive::raised(&w.capital, &w.capital.drives[k]);
    if got < drive::PUSH_BREAK * goal {
        w.capital.drives[k].crowd += drive::PUSH_BREAK * goal - got;
    }
    let words = drive::push_borrow(&mut w, mine, false).unwrap();
    println!("{words}");
    assert!(drive::push_borrow(&mut w, mine, false).is_err(), "one push a year");
    let x = w.capital.drives[k].clone();
    if x.ground.is_some() {
        assert!(x.borrowed > 0.0);
    }
    // The staff pool's cut: half the pushed bonds' yearly cost.
    w.capital.drives[k].borrowed = 60.0e6;
    w.capital.drives[k].ground = Some(w.year);
    let cut = drive::staff_cut(&w, mine, w.year, 24.0e6);
    println!("a $60M push costs the staff pool {:.0}%", 100.0 * cut);
    assert!((cut - 0.5 * 60.0e6 * drive::BOND_RATE / 24.0e6).abs() < 1e-9);
    assert_eq!(drive::staff_cut(&w, mine, w.year + drive::BOND_YEARS, 24.0e6), 0.0);
    assert!(drive::staff_cut(&w, mine, w.year, 1.0e6) <= drive::STAFF_CUT_MAX);
}

/// A cost overrun (CP24): cut rooms, wait a year and raise more, or borrow.
#[test]
fn an_overrun_is_cut_waited_out_or_borrowed() {
    for fix in [Fix::CutRooms, Fix::Wait, Fix::Borrow] {
        let (mut w, mine, _) = world();
        rich(&mut w, mine);
        w.finance.projects.retain(|x| x.owner != Owner::College(mine));
        w.today = Date::ymd(w.year, 7, 1);
        drive::start(&mut w, mine).expect("a drive");
        let k = drive::open_drive(&w.capital, mine).unwrap();
        drive::set_goal(&mut w, mine, Size::Full).unwrap();
        w.capital.drives[k].stage = Stage::Public;
        w.capital.drives[k].crowd = w.capital.drives[k].goal;
        drive::break_ground(&mut w, k, false);
        let today = w.today;
        w.capital.drives[k].overrun = Some(drive::Overrun { share: 0.2, found: today, fix: None, told: true });
        let before = w.capital.drives[k].clone();
        let cost = |w: &World| {
            w.finance
                .projects
                .iter()
                .find(|y| y.owner == Owner::College(mine) && !y.done)
                .map(|y| (y.cost, y.opens))
                .unwrap()
        };
        let (c0, o0) = cost(&w);
        let words = drive::fix_overrun(&mut w, mine, fix).unwrap();
        println!("{fix:?}: {words}");
        let x = w.capital.drives[k].clone();
        let (c1, o1) = cost(&w);
        match fix {
            Fix::CutRooms => {
                assert!(x.rooms.count_ones() < before.rooms.count_ones());
                assert_eq!((c1, o1), (c0, o0));
            }
            Fix::Wait => {
                assert_eq!(o1, o0 + 1);
                assert!(x.goal > before.goal && c1 > c0);
            }
            Fix::Borrow => {
                assert_eq!(o1, o0);
                assert!(x.borrowed > before.borrowed && c1 > c0);
            }
        }
        assert!(drive::fix_overrun(&mut w, mine, fix).is_err(), "answered once");
    }
}

/// The store keeps its new parts in a save, and an older file loads them
/// empty.
#[test]
fn the_drive_store_saves_and_an_old_file_loads_empty() {
    let (mut w, _, p) = world();
    w.today = Date::ymd(w.year, 7, 1);
    w.finance.projects.retain(|x| x.owner != Owner::College(p));
    drive::start(&mut w, p);
    let s = serde_json::to_string(&w.capital).unwrap();
    let back: capital::Capital = serde_json::from_str(&s).unwrap();
    assert_eq!(back.drives.len(), w.capital.drives.len());
    let old: capital::Capital =
        serde_json::from_str(r#"{"donors":[],"pledges":[{"id":1,"donor":2,"program":3,"total":5.0}]}"#).unwrap();
    assert!(old.drives.is_empty() && old.moments.is_empty() && old.catches.is_empty());
    assert_eq!(old.pledges[0].drive, 0);
}
