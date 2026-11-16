//! Ground lane G1b: standing and one memory (`world/ground/`).

use crate::calendar::Date;
use crate::world::World;
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::ground::{Holder, Who, people};

/// The tiny college seat with a school whose head coach is in the world.
fn world() -> (World, usize, usize, usize) {
    let w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let you = w.career.as_ref().unwrap().coach;
    let h = (0..w.high_schools.len())
        .find(|&h| w.high_schools[h].coach.is_some() && w.high_schools[h].oc.is_some())
        .or_else(|| (0..w.high_schools.len()).find(|&h| w.high_schools[h].coach.is_some()))
        .expect("a school with a head coach");
    (w, prog, you, h)
}

/// No day-one ground anywhere: no pipelines, no footprint, no ties.
fn clean(w: &mut World, prog: usize) {
    let p = &mut w.programs[prog];
    p.hs_pipeline.clear();
    p.area_pipeline.clear();
    p.footprint = [0.0; 51];
    p.pipeline = [0.0; 51];
    for c in &mut w.coaches {
        c.ties.clear();
    }
}

fn hc(w: &World, h: usize) -> Who {
    people::head_coach_of(w, h).unwrap()
}

/// A week on from a day.
fn wk(d: Date, n: i32) -> Date {
    d.add(7 * n)
}

#[test]
fn day_one_reads_the_programs_pipelines_and_its_coaches_ties() {
    let (mut w, prog, _, h) = world();
    let who = hc(&w, h);
    let ph = Holder::Program(prog as u32);
    let st = w.high_schools[h].state;
    w.programs[prog].hs_pipeline.clear();
    w.programs[prog].area_pipeline.clear();
    w.programs[prog].pipeline[st] = 0.0;
    w.programs[prog].footprint[st] = 0.0;
    assert_eq!(standing::derived(&w, who, ph), 0.0, "no ground without history");
    // Footprint and the state's pipeline alone stay under Home.
    w.programs[prog].footprint[st] = 1.0;
    w.programs[prog].pipeline[st] = 1.0;
    let area = w.high_schools[h].area;
    w.programs[prog].area_pipeline.insert(area, (1.0, 2030));
    let v = standing::derived(&w, who, ph);
    assert!(v > 50.0 && v <= 74.0, "a strong state and area without the school: {v}");
    // The school itself as a pipeline can make it Home.
    w.programs[prog].hs_pipeline.insert(h, (1.0, 2030));
    assert!(Level::of(standing::derived(&w, who, ph)) == Level::Home);
    // A coach's tie to the school is a bond with its head coach today.
    let other = (0..w.coaches.len()).find(|&c| people::coach_school(&w, c).is_none()).unwrap();
    w.coaches[other].ties.insert(h, 0.5);
    let tie = standing::derived(&w, who, Holder::Coach(other as u32));
    assert!((tie - 42.5).abs() < 1e-3, "{tie}");
    // Nobody stored anything for that: the AI's ground is read when asked.
    assert!(w.ground.memories.is_empty());
}

#[test]
fn standing_climbs_inside_the_gates() {
    let (mut w, prog, you, h) = world();
    clean(&mut w, prog);
    let who = hc(&w, h);
    let man = w.programs[prog].position_coaches.iter().flatten().copied().find(|&c| c != you);
    let d0 = w.today;
    standing::contact(&mut w, prog, man, who, Contact::Visit, 1.0, d0);
    // One contact is still a Stranger, however warm.
    assert_eq!(standing::level(&w, prog, who), Level::Stranger);
    // Thirty visits in a month: Known, but a year short of Trusted (GR28).
    for k in 1..30 {
        standing::contact(&mut w, prog, man, who, Contact::Visit, 1.0, d0.add(k));
    }
    let (pts, _) = standing::staff(&w, prog, who);
    assert_eq!(Level::of(pts), Level::Known, "{pts}");
    assert!(pts <= 49.9);
    // A year on, with visits kept up, Trusted; never Home before three years.
    for k in 0..60 {
        standing::contact(&mut w, prog, man, who, Contact::Visit, 1.0, wk(d0, 5 + k));
    }
    let (pts, holder) = standing::staff(&w, prog, who);
    assert_eq!(Level::of(pts), Level::Trusted, "{pts}");
    // The man who did the work holds the most; the program a third.
    if let Some(c) = man {
        assert_eq!(holder, Holder::Coach(c as u32));
        let program = standing::points(&w, who, Holder::Program(prog as u32));
        assert!(program < pts && program > 0.0, "program {program} man {pts}");
    }
    let m = w.ground.get(who, holder).unwrap();
    assert!(m.known_since.is_some());
    // He said each new level in his own words, at his hour.
    let mails: Vec<_> =
        w.career.as_ref().unwrap().desk.mails.iter().filter(|m| m.head == crate::world::desk::Head::Email).collect();
    assert!(mails.iter().any(|m| m.body.iter().any(|b| b.contains("give you my list"))), "Known came as a letter");
    assert!(mails.iter().any(|m| m.body.iter().any(|b| b.contains("what I really think of my kids"))), "Trusted too");
    assert!(mails.iter().all(|m| (13 * 60..14 * 60).contains(&m.minute)), "a coach writes after lunch");
    assert!(m.moments.iter().any(|x| x.words.contains("what I really think")));
    w.today = wk(d0, 64);
    let why = standing::why(&w, prog, who);
    assert!(why.starts_with("Trusted: ") && why.contains("this year"), "{why}");
}

#[test]
fn a_second_visit_in_a_week_buys_half_and_the_head_coachs_call_counts_by_his_name() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let who = hc(&w, h);
    let ph = Holder::Program(prog as u32);
    // Personnel staff (nobody's name): the program holds it all.
    let d = Date::ymd(2037, 4, 6); // a Monday
    standing::contact(&mut w, prog, None, who, Contact::Visit, 1.0, d);
    let first = standing::points(&w, who, ph);
    standing::contact(&mut w, prog, None, who, Contact::Visit, 1.0, d.add(2));
    let second = standing::points(&w, who, ph) - first;
    assert!((second - first / 2.0).abs() < 1e-3, "first {first} second {second}");
    // A head coach's call: about 2 visits at a top program, about 1 at a small one.
    let head = w.programs[prog].head_coach.unwrap();
    let skill = people::skill(&w, Holder::Coach(head as u32));
    let mut size = |prestige: f64, man: Option<usize>, kind: Contact| -> f32 {
        w.programs[prog].prestige = prestige;
        w.ground = Default::default();
        standing::contact(&mut w, prog, man, who, kind, 1.0, d);
        standing::points(&w, who, man.map_or(ph, |c| Holder::Coach(c as u32)))
    };
    for (p, want) in [(0.9, 2.0), (0.2, 1.0)] {
        let call = size(p, Some(head), Contact::Call);
        let visit = size(p, None, Contact::Visit);
        let r = call / visit / skill;
        assert!((r - want).abs() < 0.2, "prestige {p}: a head coach's call is {r} visits");
    }
}

#[test]
fn standing_fades_warns_and_a_pulled_offer_is_held_for_years() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let who = hc(&w, h);
    let ph = Holder::Program(prog as u32);
    let d0 = Date::ymd(2037, 3, 1);
    standing::day(&mut w, d0); // day one
    // Known with the program, last seen in early March.
    {
        let m = w.ground.entry(who, ph, || crate::world::ground::Memory::new(who, ph, 0.0, d0));
        m.points = 25.5;
        m.contacts = 3;
        m.known_since = Some(d0.add(-30));
        m.last = d0;
    }
    w.ground.set_told(who, Level::Known as u8);
    // April 1 (31 days): no fade yet, but the month ahead drops it: a warning.
    let before = w.career.as_ref().unwrap().desk.mails.len();
    standing::day(&mut w, Date::ymd(2037, 4, 1));
    let p1 = standing::points(&w, who, ph);
    assert_eq!(p1, 25.5, "no fade inside 60 days");
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.len() > before, "a warning came");
    let warn = mails.iter().rev().find(|m| m.subject.ends_with("is slipping")).expect("a slipping memo");
    assert!(
        warn.body[0].contains("no visit since March") && warn.body[0].contains("A visit this month holds it"),
        "{:?}",
        warn.body
    );
    // May 1 (61 days): -1.
    standing::day(&mut w, Date::ymd(2037, 5, 1));
    let p2 = standing::points(&w, who, ph);
    assert!(p1 - p2 >= 1.0 - 1e-3, "{p1} -> {p2}");
    // A pulled offer: -20 and a big moment held for years.
    let kid = w.people.iter().position(|q| q.hs == h && q.stage == crate::world::Stage::HighSchool);
    if let Some(i) = kid {
        let id = w.people[i].player.id;
        let d = Date::ymd(2037, 5, 3);
        w.front_office.college.board.push(crate::world::front_office::college::Target {
            id,
            added: d,
            by: crate::world::front_office::college::Source::You,
            offered: Some(d.add(-30)),
            pulled: Some(d),
            priority: false,
            nil: None,
            nil_coll: None,
            years: 0,
            why: None,
            seen: Default::default(),
        });
        let before = standing::points(&w, who, ph);
        standing::day(&mut w, d);
        let after = standing::points(&w, who, ph);
        assert!((before - after - 20.0).abs() < 1e-3 || after == 0.0, "{before} -> {after}");
        let m = w.ground.get(who, ph).unwrap();
        let held = m.holding(d.add(365 * 2)).expect("held two years on");
        assert!(held.words.contains("pulled") && held.words.contains("May"), "{}", held.words);
        assert!(m.holding(d.add(365 * 6)).is_none(), "gone by six years");
        // Paid once.
        standing::day(&mut w, d.add(1));
        assert_eq!(standing::points(&w, who, ph), after);
        // While he holds it, a visit buys a quarter.
        let p = standing::points(&w, who, ph);
        standing::contact(&mut w, prog, None, who, Contact::Visit, 1.0, d.add(10));
        let got = standing::points(&w, who, ph) - p;
        let mk = people::makeup(&w, who);
        let full = standing::VISIT * mk.warmth * (0.9 + 0.2 * w.programs[prog].prestige as f32);
        assert!((got - full * standing::HELD).abs() < 1e-2, "{got} vs {}", full * standing::HELD);
    }
}

#[test]
fn your_roots_are_ground_on_day_one_and_your_own_ties_go_with_you() {
    let (mut w, prog, you, h) = world();
    let town = w.high_schools[h].town.clone();
    let state = w.high_schools[h].state;
    let neighbor = (0..w.high_schools.len()).find(|&x| {
        x != h
            && w.high_schools[x].state == state
            && w.high_schools[x].town == town
            && w.high_schools[x].coach.is_some()
    });
    w.career.as_mut().unwrap().past = Some(crate::world::front_office::market::Placement {
        college: None,
        played: false,
        high_school: Some((h, 2000, 2003)),
        stops: Vec::new(),
    });
    let today = w.today;
    standing::day(&mut w, today);
    let who = hc(&w, h);
    let yh = Holder::Coach(you as u32);
    assert_eq!(Level::of(standing::points(&w, who, yh)), Level::Trusted, "your own high school's coach");
    assert!(w.ground.get(who, yh).unwrap().moments.iter().any(|m| m.words == "You're one of ours."));
    if let Some(n) = neighbor {
        assert_eq!(Level::of(standing::points(&w, hc(&w, n), yh)), Level::Known, "a hometown coach");
    }
    let kid = w.people.iter().position(|q| q.hs == h).unwrap();
    assert!(standing::knows_your_name(&w, kid));
    // The staff's level is the best of the program and its men (GR29).
    assert!(standing::level(&w, prog, who) >= Level::Trusted);
    assert!(standing::why(&w, prog, who).contains("you played for his school"));
    // A new job: the old program's memories stay behind, yours come along.
    let ph = Holder::Program(prog as u32);
    let today = w.today;
    standing::contact(&mut w, prog, None, who, Contact::Visit, 1.0, today);
    assert!(w.ground.get(who, ph).is_some());
    let other = (0..w.programs.len()).find(|&p| p != prog).unwrap();
    w.seat.as_mut().unwrap().program = Some(other);
    let today = w.today;
    standing::day(&mut w, today);
    assert!(w.ground.get(who, ph).is_none(), "the program's ties stay with it");
    assert!(w.ground.get(who, yh).is_some(), "your own ties go with you");
}

#[test]
fn an_areas_level_weights_its_people_by_the_talent_they_control() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let other = (0..w.high_schools.len()).find(|&x| x != h && w.high_schools[x].coach.is_some()).unwrap();
    let ph = Holder::Program(prog as u32);
    for (x, p) in [(h, 80.0), (other, 0.0)] {
        let who = hc(&w, x);
        let d = w.today;
        let m = w.ground.entry(who, ph, || crate::world::ground::Memory::new(who, ph, 0.0, d));
        m.points = p;
    }
    w.ground.seeded = Some((w.career.as_ref().unwrap().coach as u32, prog as u32));
    let mut t = vec![1.0f32; w.high_schools.len()];
    t[h] = 9.0;
    let a = standing::area_points(&w, prog, &[h, other], &t);
    assert!((a - 72.0).abs() < 1.0, "the school with the talent decides: {a}");
    t[h] = 1.0;
    t[other] = 9.0;
    let b = standing::area_points(&w, prog, &[h, other], &t);
    assert!(b < 12.0, "{b}");
    assert_eq!(standing::talents(&w).len(), w.high_schools.len());
}

#[test]
fn the_ground_store_saves_and_an_old_file_loads_empty() {
    let (mut w, prog, _, h) = world();
    let today = w.today;
    standing::day(&mut w, today);
    let who = hc(&w, h);
    standing::contact(&mut w, prog, None, who, Contact::Visit, 1.0, today);
    let bytes = rmp_serde::to_vec_named(&w.ground).unwrap();
    let back: crate::world::ground::Ground = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.memories.len(), w.ground.memories.len());
    let old: crate::world::ground::Ground = serde_json::from_str("{}").unwrap();
    assert!(old.memories.is_empty());
    println!("ground store: {} memories, {} bytes", w.ground.memories.len(), bytes.len());
}

/// The world plays the same with the ground on: it only writes its own
/// store, your inbox and (lane G7) your own network's ground people.
#[test]
fn the_world_plays_the_same_with_the_ground() {
    // Values, so the maps compare by key whatever their order in memory.
    let fp = |w: &World| -> Vec<serde_json::Value> {
        vec![
            serde_json::to_value(&w.people).unwrap(),
            serde_json::to_value(&w.programs).unwrap(),
            serde_json::to_value(&w.coaches).unwrap(),
            serde_json::to_value(&w.scouting.dept.looks).unwrap(),
            serde_json::to_value(&w.folk).unwrap(),
        ]
    };
    let mut on = super::legacy::seat("college");
    let mut off = super::legacy::seat("college");
    off.ground.off = true;
    assert!(fp(&on) == fp(&off), "two seats from one seed start the same");
    // Through the spring evaluation's first Thursdays.
    let start = on.today;
    let end = if start < Date::ymd(start.parts().0, 5, 20) { Date::ymd(start.parts().0, 5, 20) } else { start.add(90) };
    println!("from {} to {}", start.pretty(), end.pretty());
    // The tiny world's board is empty, so both worlds get the same history
    // and the same looks by hand: a pipeline school, and a coach at a kid's
    // practice every Thursday through the real hook.
    let prog = on.seat.and_then(|s| s.program).unwrap();
    // The seat world has no high schoolers: six college players go back to
    // high school in both worlds alike.
    let year = on.today.parts().0;
    let picks: Vec<usize> = (0..on.people.len()).filter(|&i| on.people[i].college != Some(prog)).take(6).collect();
    let kids: Vec<u32> = picks.iter().map(|&i| on.people[i].player.id).collect();
    for w in [&mut on, &mut off] {
        for (k, &i) in picks.iter().enumerate() {
            let q = &mut w.people[i];
            q.stage = crate::world::Stage::HighSchool;
            q.college = None;
            q.class_of = year + 1;
            q.hs = k % 3;
        }
    }
    assert!(!kids.is_empty());
    let coach = on.programs[prog].position_coaches.iter().flatten().copied().next().unwrap_or(0);
    for w in [&mut on, &mut off] {
        let h = w.people[crate::world::scouting::place(w, kids[0]).unwrap()].hs;
        w.programs[prog].hs_pipeline.insert(h, (0.9, 2024));
    }
    while on.today < end {
        if on.today.weekday() == crate::calendar::Weekday::Thu {
            for w in [&mut on, &mut off] {
                let d = w.today;
                for &id in &kids {
                    crate::world::scouting::dept::add(
                        w,
                        id,
                        crate::world::scouting::readers::COACH + coach as u32,
                        crate::world::scouting::Kind::Practice,
                        d,
                    );
                }
            }
        }
        on.advance_day();
        off.advance_day();
    }
    // Everything but your own network plays the same; in your network only
    // the people on your ground differ, since their ties read your
    // standing with them (lane G7, `ground::stores`). And which of your
    // own men made a look differs: the man on the area goes (lane G10,
    // `ground::staff::beats`), always one of your staff.
    let yours: Vec<u64> = crate::world::ground::staff::men(&on, prog)
        .iter()
        .map(|&c| (crate::world::scouting::readers::COACH + c as u32) as u64)
        .collect();
    let strip = |mut v: Vec<serde_json::Value>| {
        v[4].as_object_mut().unwrap().remove("network");
        for list in v[3].as_object_mut().unwrap().values_mut() {
            for l in list.as_array_mut().unwrap() {
                let l = l.as_object_mut().unwrap();
                if l.get("who").and_then(|x| x.as_u64()).is_some_and(|x| yours.contains(&x)) {
                    l.remove("who");
                }
            }
        }
        v
    };
    let (fa, fb) = (strip(fp(&on)), strip(fp(&off)));
    assert!(fa == fb, "the world moved with the ground");
    let (a, b) = (&on.folk.network.contacts, &off.folk.network.contacts);
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b) {
        assert_eq!((x.coach, x.other, &x.how), (y.coach, y.other, &y.how));
        if crate::world::ground::stores::ground_who(&on, x).is_none() {
            assert_eq!(serde_json::to_value(x).unwrap(), serde_json::to_value(y).unwrap(), "{}", x.how);
        }
    }
    let kinds: Vec<String> = on.scouting.dept.looks.values().flatten().map(|l| format!("{:?}", l.kind)).collect();
    println!("looks: {} {:?}", kinds.len(), kinds.iter().take(20).collect::<Vec<_>>());
    assert!(off.ground.memories.is_empty());
    assert!(!on.ground.memories.is_empty(), "the seat's ground was read");
    println!(
        "ground after {}: {} memories, {} bytes",
        on.today.pretty(),
        on.ground.memories.len(),
        rmp_serde::to_vec_named(&on.ground).unwrap().len()
    );
}
