//! The recruit and portal conversation on the fact sheet (talk facts lane
//! L5; `world::folk::{case, pitch, pulls}`): the case's cap and comebacks,
//! the talk's flow, the proof that your best talk flips only close races
//! and never down a level, and the proof that after three talks your file
//! names what pulls him more often than the free guess alone, and more
//! often the better you read people (talk facts design, section 10).

use super::legacy::seat;
use crate::calendar::Date;
use crate::world::data::Division;
use crate::world::folk::case::{self, CASE_FLOOR, CASE_TOP, Case, SIT_DOWN_EXCHANGES};
use crate::world::folk::claims::{self, Ear, Group, Kind, Tier};
use crate::world::folk::scene::{self, Kind as Scenes, PickKind, Scene};
use crate::world::folk::{dialogue, pitch, pulls};
use crate::world::scouting::{self, SeatLook};
use crate::world::{Stage, World, recruiting};

const POINT: std::ops::Range<u32> = 7100..7300;
const GROUP: std::ops::Range<u32> = 7500..7507;
const LEAVE: u32 = 719;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-6
}

#[test]
fn a_case_counts_each_group_once_and_comebacks_less_under_one_cap() {
    let d = Date::ymd(2037, 1, 10);
    let mut c = Case::new(3);
    assert!(near(c.add(Group::Money, 0.02, 1.0, d), 0.02));
    // The same group again: only a better point moves it.
    assert!(near(c.add(Group::Money, 0.01, 1.0, d), 0.0));
    assert!(near(c.add(Group::Money, 0.025, 1.0, d), 0.005));
    // A point that landed, brought back in a later talk: half, then a
    // quarter; nothing in the same talk.
    assert!(near(c.add(Group::Money, 0.02, 0.5, d), 0.01));
    assert!(near(c.add(Group::Money, 0.02, 0.25, d), 0.005));
    assert!(near(c.add(Group::Money, 0.02, 0.0, d), 0.0));
    // A point he heard for another case (a school you left) counts in full
    // in a case with nothing yet in its group.
    assert!(near(c.add(Group::Home, 0.01, 0.5, d), 0.01));
    assert!(c.has(Group::Home));
    // The points a case has heard, one bit each, kept through a merge.
    assert!(Kind::ALL.len() <= 256, "a case keeps 256 points' bits");
    let last = Kind::ALL.len() - 1;
    assert!(!c.heard(last));
    c.hear(last);
    let mut m = Case::new(3);
    m.merge(&c);
    assert!(m.heard(last) && !m.heard(0));
    // Every group at its best still stops at the cap.
    for g in Group::ALL {
        c.add(g, 0.03, 1.0, d);
    }
    assert!(near(c.total(), CASE_TOP));
    // A case of points he doubted bottoms out.
    let mut b = Case::new(4);
    for g in Group::ALL {
        b.add(g, -0.012, 1.0, d);
    }
    assert!(near(b.total(), CASE_FLOOR));
    // A second scene folds in each group at the better of the two: more
    // scenes never raise the ceiling.
    let mut x = Case::new(5);
    x.add(Group::School, 0.01, 1.0, d);
    let mut y = Case::new(5);
    y.add(Group::School, 0.02, 1.0, d);
    y.add(Group::Home, 0.005, 1.0, d);
    x.merge(&y);
    assert!(near(x.total(), 0.025));
}

/// Your FBS program and `n` high school seniors, each offered by three
/// other colleges (a fixed spread over FBS, FCS and Division II, or with
/// `near` the three that mean nearest to what your school means to him, as
/// real offers come from schools like yours), each of those schools' head
/// coach in his living room once.
fn recruits(n: usize) -> (World, usize, Vec<usize>) {
    recruits_with(n, false)
}

fn recruits_with(n: usize, near: bool) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(17);
    let class = recruiting::next_class(&w);
    let others: Vec<usize> = (0..w.programs.len())
        .filter(|&j| j != mine && matches!(w.programs[j].division, Division::FBS | Division::FCS | Division::D2))
        .collect();
    let kids: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    let today = w.today;
    for (k, &i) in kids.iter().enumerate() {
        w.people[i].class_of = class;
        let list: Vec<usize> = if near {
            let a = recruiting::appeals(&w, i, &others, None);
            let me = recruiting::appeals(&w, i, &[mine], None)[0];
            let mut v: Vec<(f64, usize)> = a.iter().zip(&others).map(|(x, &j)| ((x - me).abs(), j)).collect();
            v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            v.iter().take(3).map(|x| x.1).collect()
        } else {
            (1..=3).map(|x| others[(k * 7 + x * 41) % others.len()]).collect()
        };
        let p = &mut w.people[i];
        p.cal.offers = list.clone();
        for j in list {
            dialogue::ai_home_visit(&mut w, i, j, today);
        }
    }
    (w, mine, kids)
}

/// A home visit with him today, opened.
fn visit(w: &mut World, i: usize, mine: usize) -> Scene {
    let id = w.people[i].player.id;
    let today = w.today;
    dialogue::home_visit_set(w, id, mine, today);
    let s = w.folk.scenes.iter().rev().find(|s| s.kind == Scenes::HomeVisit && s.a == id as u64 && !s.done).unwrap().id;
    scene::open(w, s).expect("the visit opens")
}

/// What a point pick would add to your case, read from the truth: how
/// much he cares, your proof, and what the case already holds.
fn adds(w: &World, i: usize, mine: usize, code: u32) -> f32 {
    let Some(&k) = Kind::ALL.get((code - POINT.start) as usize) else { return 0.0 };
    let sheet = claims::recruit(w, i, mine, Ear::Decider);
    let Some(pt) = sheet.get(k) else { return 0.0 };
    let Some(g) = k.group() else { return 0.0 };
    let you = w.career.as_ref().map(|c| c.coach);
    let lean = case::lands(w, pt.care, pt.tier().unwrap_or(Tier::Fair), you);
    let mut c = case::of(&w.people[i], mine).copied().unwrap_or(Case::new(mine));
    let place = Kind::ALL.iter().position(|&x| x == k).unwrap();
    let again = if c.heard(place) { w.folk.talked.again(w.today, w.people[i].player.id, pt) } else { 1.0 };
    let before = c.total();
    c.add(g, lean, again, w.today);
    c.total() - before
}

/// Your talk at its best: each exchange the point that adds most, from
/// the answers shown and every point behind "bring up something else";
/// you leave once nothing adds.
fn best_talk(w: &mut World, i: usize, mine: usize) -> Scene {
    let mut s = visit(w, i, mine);
    for _ in 0..40 {
        if s.done {
            break;
        }
        let best = s
            .picks
            .iter()
            .chain(&s.reach)
            .filter(|p| POINT.contains(&p.code))
            .map(|p| (adds(w, i, mine, p.code), p.code))
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)));
        let code = match best {
            Some((x, c)) if x > 0.0 => c,
            _ => LEAVE,
        };
        s = scene::answer(w, s.id, code).expect("an answer on the table");
    }
    assert!(s.done, "the talk ends");
    s
}

/// Who leads his race, and every school's appeal.
fn race(w: &World, i: usize, mine: usize) -> (usize, Vec<(usize, f64)>) {
    let mut schools = w.people[i].cal.offers.clone();
    schools.push(mine);
    let a = recruiting::appeals(w, i, &schools, None);
    let v: Vec<(usize, f64)> = schools.into_iter().zip(a).collect();
    let lead = v.iter().max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0))).unwrap().0;
    (lead, v)
}

#[test]
fn a_home_visit_opens_asks_and_offers_every_point() {
    let (mut w, mine, kids) = recruits(12);
    let mut talked = 0;
    for &i in &kids {
        let s = visit(&mut w, i, mine);
        assert!(s.lines.iter().any(|l| l.by == 2 && !l.text.is_empty()), "his side speaks first");
        if s.done {
            continue;
        }
        talked += 1;
        let codes: Vec<u32> = s.picks.iter().map(|p| p.code).collect();
        // The free questions use no exchange; the way around is a nav.
        for c in [709, 7601, 7602, 7603] {
            let p = s.picks.iter().find(|p| p.code == c).unwrap_or_else(|| panic!("free question {c}: {codes:?}"));
            assert_eq!(p.kind, PickKind::Free);
        }
        assert!(s.picks.iter().any(|p| p.code == LEAVE));
        assert!(
            s.picks
                .iter()
                .any(|p| POINT.contains(&p.code) || GROUP.contains(&p.code) || (7530..7537).contains(&p.code)),
            "an answer to his question or the floor: {codes:?}"
        );
        assert!(s.picks.iter().filter(|p| GROUP.contains(&p.code) || p.code == 7510).all(|p| p.kind == PickKind::Nav));
        // Every point he can hear today is shown or behind "something else".
        let sheet = claims::recruit(&w, i, mine, Ear::Decider);
        let known = sheet.points.iter().filter(|p| p.known && matches!(p.kind.family(), claims::Family::Pitch(_)));
        for pt in known {
            let code = POINT.start + Kind::ALL.iter().position(|&k| k == pt.kind).unwrap() as u32;
            assert!(s.picks.iter().chain(&s.reach).any(|p| p.code == code), "{:?} is out of reach", pt.kind);
        }
        // Your words for every point are whole: no fact left empty.
        for p in s.picks.iter().chain(&s.reach) {
            let l = &p.label;
            assert!(
                !l.contains(['{', '}']) && ![" .", " :", " ,", "  ", "at ."].iter().any(|x| l.contains(x)),
                "a pick with a fact missing: {l}"
            );
        }
        // Asking what he's looking for costs no exchange; leaving after
        // only listening adds nothing (no listening bonus).
        let s = scene::answer(&mut w, s.id, 709).unwrap();
        assert!(!s.done);
        let s = scene::answer(&mut w, s.id, LEAVE).unwrap();
        assert!(s.done);
        assert!(case::of(&w.people[i], mine).is_none_or(|c| c.total() == 0.0), "listening alone moves nothing");
        for l in &s.lines {
            assert!(!l.text.contains(['{', '}']), "a fact left unfilled: {}", l.text);
        }
    }
    assert!(talked > 0, "some visit had something to talk about");
}

#[test]
fn a_sit_down_runs_its_exchanges_and_a_handed_one_is_the_coachs_case() {
    let (mut w, mine, kids) = recruits(12);
    let mut full = 0;
    for &i in &kids {
        let mut s = visit(&mut w, i, mine);
        let mut made = 0;
        while !s.done && made < 20 {
            let code = s.picks.iter().chain(&s.reach).find(|p| POINT.contains(&p.code)).map_or(LEAVE, |p| p.code);
            made += usize::from(code != LEAVE);
            s = scene::answer(&mut w, s.id, code).unwrap();
        }
        assert!(s.done);
        if made == SIT_DOWN_EXCHANGES {
            full += 1;
        }
        assert!(made <= SIT_DOWN_EXCHANGES, "a sit-down is {SIT_DOWN_EXCHANGES} exchanges, not {made}");
        let c = case::of(&w.people[i], mine).copied().unwrap_or_default();
        assert!(c.total() <= CASE_TOP && c.total() >= CASE_FLOOR);
    }
    assert!(full > 0, "some talk used every exchange");
    // Handed to your staff: the position coach makes the case in one step
    // and brings back what he asked, from his own read.
    let i = kids[0];
    let id = w.people[i].player.id;
    w.today = w.today.add(7);
    let today = w.today;
    dialogue::home_visit_set(&mut w, id, mine, today);
    let sid = w.folk.scenes.iter().rev().find(|s| s.a == id as u64 && !s.done).unwrap().id;
    let s = scene::hand(&mut w, sid).unwrap();
    assert!(s.done && s.handed.is_some());
    assert!(w.folk.pulls.of(id).any(|r| r.from == pulls::Where::Staff), "the staff's rows are on his file");
    assert!(
        !w.folk.pulls.of(id).any(|r| r.on == today && r.from != pulls::Where::Staff),
        "a talk you never opened puts nothing of yours on his file"
    );
    // One talk with him at a time: start another and the one you left open
    // ends as it stands.
    w.today = w.today.add(7);
    let mut checked = 0;
    for &i in &kids[1..] {
        let first = visit(&mut w, i, mine);
        let Some(code) = first.picks.iter().chain(&first.reach).find(|p| POINT.contains(&p.code)).map(|p| p.code)
        else {
            continue;
        };
        let first = scene::answer(&mut w, first.id, code).unwrap();
        if first.done {
            continue;
        }
        let before = case::pull(&w.people[i], mine);
        let second = visit(&mut w, i, mine);
        let was = w.folk.scenes.iter().find(|x| x.id == first.id).unwrap();
        assert!(was.done && was.handed.as_deref() == Some("nobody"), "the first talk ended");
        assert!(was.outcome.iter().any(|o| o.contains("stands as you left it")));
        assert!(second.opened);
        assert!((case::pull(&w.people[i], mine) - before).abs() < 1e-9, "leaving it adds nothing");
        checked += 1;
    }
    assert!(checked > 0, "some talk was left open and ended by the next");
}

/// The proof the design asks for (section 10): the same recruits, the same
/// day, your visit handed to staff against your talk at its best. Only
/// your school's case changes, so a recruit flips only where your school
/// and his leader were within what a case can move (the cap and the
/// floor), and a talk never pulls him down a level (section 6, "How much
/// it counts": a power school's lead over a Group of 5 school is about 0.2
/// from the level alone). The world: the seat test world, 120 recruits,
/// each offered by the three schools that mean nearest to what yours means
/// to him, so most races are close and some cross a level; one visit each
/// per arm (a few seconds).
#[test]
fn your_best_talk_flips_only_close_races_and_never_down_a_level() {
    let (mut w, mine, kids) = recruits_with(120, true);
    // 0 a power school, 1 the rest of FBS, 2 FCS, 3 Division II.
    let level = |w: &World, j: usize| match w.programs[j].division {
        Division::FBS if w.programs[j].power_conference(w.year) => 0,
        Division::FBS => 1,
        Division::FCS => 2,
        _ => 3,
    };
    let (mut flips, mut close, mut up) = (0, 0, 0);
    for &i in &kids {
        let before = w.people[i].cal.cases.clone();
        let s = visit(&mut w, i, mine);
        let s = if s.done { s } else { scene::hand(&mut w, s.id).unwrap() };
        assert!(s.done);
        let (a, va) = race(&w, i, mine);
        let handed = case::pull(&w.people[i], mine);
        w.people[i].cal.cases = before;
        best_talk(&mut w, i, mine);
        let (b, _) = race(&w, i, mine);
        let best = case::pull(&w.people[i], mine);
        let at = |j: usize| va.iter().find(|x| x.0 == j).unwrap().1;
        let gap = (at(a) - at(mine)).abs();
        if a != mine && gap <= (CASE_TOP - CASE_FLOOR) as f64 {
            close += 1;
        }
        if a != b {
            flips += 1;
            println!(
                "{}: {} (level {}) to {} (level {}), {gap:.3} apart, the talks moved {:.3}",
                w.people[i].name,
                w.program_name(a),
                level(&w, a),
                w.program_name(b),
                level(&w, b),
                best - handed
            );
            assert!(
                gap <= (best - handed).abs() + 1e-9,
                "{} flipped across a gap of {gap:.3} that the talks moved {:.3}",
                w.people[i].name,
                best - handed
            );
            // Toward the school his talk was better for: never a level down.
            let (from, to) = if best > handed { (a, b) } else { (b, a) };
            assert!(
                level(&w, to) <= level(&w, from),
                "{} was talked down a level ({} to {})",
                w.people[i].name,
                w.program_name(from),
                w.program_name(to)
            );
            up += usize::from(level(&w, to) < level(&w, from));
        }
    }
    println!("{} recruits, {close} close races with you behind, {flips} flips, {up} of them up a level", kids.len());
    assert!(flips > 0, "with schools like yours on his list, your talks decide some close races");
}

/// The proof of section 7: after three talks the file names what pulls him
/// most more often than your area recruiter's free guess alone, and more
/// often the better you read people. You play each talk as a coach would:
/// in the first you ask what he's looking for, then you answer each of his
/// questions with your best proof (straight when you have nothing there),
/// and given the floor you bring up your groups in turn.
#[test]
fn after_three_talks_your_file_beats_the_free_guess_and_reading_people_helps() {
    let run = |reading: f64| -> (usize, usize, usize) {
        let (mut w, mine, kids) = recruits(120);
        w.career.as_mut().unwrap().you.relationships = reading;
        // Your area recruiter's call to his coaches: the free guess.
        let today = w.today;
        for &i in &kids {
            let id = w.people[i].player.id;
            let c = dialogue::caller(&w, i, mine).expect("a staff");
            w.scouting.dept.looks.entry(id).or_default().push(SeatLook {
                who: scouting::readers::COACH + c as u32,
                kind: scouting::Kind::Call,
                date: today,
            });
        }
        let truth = |w: &World, i: usize| {
            let c = claims::group_cares(&w.people[i], pulls::deciding_ear(w, i));
            Group::ALL.into_iter().max_by(|a, b| c[*a as usize].total_cmp(&c[*b as usize]).then(b.cmp(a))).unwrap()
        };
        let guessed = kids.iter().filter(|&&i| pulls::guess(&w, i).is_some_and(|g| g.1 == truth(&w, i))).count();
        for talk in 0..3 {
            for (n, &i) in kids.iter().enumerate() {
                let mut s = visit(&mut w, i, mine);
                if talk == 0 && !s.done {
                    s = scene::answer(&mut w, s.id, 709).unwrap();
                }
                let mut room = n + talk;
                for _ in 0..40 {
                    if s.done {
                        break;
                    }
                    let has = |r: std::ops::Range<u32>| s.picks.iter().map(|p| p.code).find(|c| r.contains(c));
                    let code = if let Some(c) = has(POINT) {
                        c
                    } else if let Some(c) = has(7530..7537) {
                        c
                    } else {
                        let groups: Vec<u32> = s.picks.iter().map(|p| p.code).filter(|c| GROUP.contains(c)).collect();
                        room += 1;
                        match groups.get(room % groups.len().max(1)) {
                            Some(&c) => c,
                            None if s.picks.iter().any(|p| p.code == 7510) => 7510,
                            None => LEAVE,
                        }
                    };
                    s = scene::answer(&mut w, s.id, code).unwrap();
                }
                assert!(s.done, "the talk ends");
            }
            w.today = w.today.add(7);
        }
        let mut miss = std::collections::BTreeMap::new();
        let mut filed = 0;
        // Right and asked, for recruits who keep it close and the rest.
        let mut by = [(0, 0); 2];
        for &i in &kids {
            let t = truth(&w, i);
            let f = pulls::file(&w, i);
            let close = crate::world::folk::roll(&w, 0x6A2D, w.people[i].player.id as u64) < pitch::GUARDED_SHARE;
            by[close as usize].1 += 1;
            if f.first().is_some_and(|r| r.rows > 0 && r.group == t) {
                filed += 1;
                by[close as usize].0 += 1;
            } else {
                *miss.entry(format!("{:?} read as {:?}", t, f.first().map(|r| r.group))).or_insert(0) += 1;
            }
        }
        println!("relationships {reading}: open {:?}, guarded {:?}; missed {miss:?}", by[0], by[1]);
        (guessed, filed, kids.len())
    };
    let (guess, poor, n) = run(3.0);
    let (_, good, _) = run(18.0);
    println!("of {n}: the free guess alone {guess}, the file after three talks {poor} (poor read), {good} (good read)");
    assert!(poor > guess, "the file ({poor}) should beat the free guess alone ({guess})");
    assert!(good > poor, "a good read ({good}) should beat a poor one ({poor})");
}
