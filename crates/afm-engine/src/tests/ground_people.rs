//! Ground lane G7: people in full (`world/ground/`): families and brothers,
//! word that travels, jobs that carry standing, the older stores reading
//! through the one memory, each person's page.

use crate::calendar::Date;
use crate::world::World;
use crate::world::coach::{Employer, Role};
use crate::world::ground::standing::{self, Contact, Level};
use crate::world::ground::{Holder, Memory, Who, card, jobs, people, stores, word};
use crate::world::scouting::{Kind, readers};

/// The tiny college seat with a school whose head coach and a coordinator
/// are in the world.
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

/// No day-one ground anywhere.
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

/// `n` players sent back to high school at `h` (the seat world has none).
fn kids(w: &mut World, prog: usize, h: usize, n: usize) -> Vec<usize> {
    let year = w.today.parts().0;
    let picks: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].college != Some(prog)).take(n).collect();
    for &i in &picks {
        let q = &mut w.people[i];
        q.stage = crate::world::Stage::HighSchool;
        q.college = None;
        q.class_of = year + 1;
        q.hs = h;
        q.stars = 3;
        w.high_schools[h].roster.push(i);
    }
    picks
}

fn hc(w: &World, h: usize) -> Who {
    people::head_coach_of(w, h).unwrap()
}

/// One of your men who isn't you.
fn man(w: &World, prog: usize, you: usize) -> usize {
    w.programs[prog].position_coaches.iter().flatten().copied().find(|&c| c != you).unwrap_or(you)
}

#[test]
fn a_call_reaches_his_family_and_the_younger_brother_reads_the_same_memory() {
    let (mut w, prog, you, h) = world();
    clean(&mut w, prog);
    let k = kids(&mut w, prog, h, 2);
    let (a, b) = (k[0], k[1]);
    // The younger one: the same surname, two classes later, an older
    // brother in college.
    let last = crate::world::names::surname_of(&w.people[a].name).to_string();
    w.people[b].name = format!("Jalen {last}");
    w.people[b].class_of = w.people[a].class_of + 2;
    w.people[b].family.older_brother_at = Some(prog);
    let fam = people::family_of(&w, a);
    assert_eq!(people::family_of(&w, b), fam, "one family for both brothers");
    assert_eq!(fam, Who::Family(w.people[a].player.id));
    let today = w.today;
    standing::day(&mut w, today);
    let id = w.people[a].player.id;
    let code = readers::COACH + man(&w, prog, you) as u32;
    for d in 0..6 {
        crate::world::scouting::dept::add(&mut w, id, code, Kind::Call, today.add(7 * d));
    }
    let (pts, _) = standing::staff(&w, prog, fam);
    assert!(pts > 0.0, "calls reach his family");
    assert!(w.ground.with(fam).iter().all(|m| m.contacts >= 1));
    // The younger brother's family is the same memory.
    assert_eq!(standing::staff(&w, prog, people::family_of(&w, b)).0, pts);
    assert!(people::family_voice(&w, fam).starts_with("his "), "{}", people::family_voice(&w, fam));
    assert!(people::name(&w, fam).ends_with("family"));
}

#[test]
fn a_son_who_graduated_makes_his_family_home_and_a_pulled_offer_is_remembered() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let k = kids(&mut w, prog, h, 2);
    let (a, b) = (k[0], k[1]);
    let d0 = Date::ymd(2037, 3, 2);
    standing::day(&mut w, d0);
    // He played for you and graduated.
    {
        let q = &mut w.people[a];
        q.stage = crate::world::Stage::College;
        q.college = Some(prog);
        q.cal.graduated = true;
    }
    let fam = people::family_of(&w, a);
    standing::day(&mut w, Date::ymd(2037, 5, 20));
    assert_eq!(standing::level(&w, prog, fam), Level::Home, "{}", standing::why(&w, prog, fam));
    // His letter came, in the family's words.
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.iter().any(|m| m.from.ends_with("family")), "the family wrote");
    // The other kid's offer is pulled: his family holds it, and reads you
    // by it before the trail's name.
    let id = w.people[b].player.id;
    let d = Date::ymd(2037, 6, 3);
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
    standing::day(&mut w, d);
    let fb = people::family_of(&w, b);
    let m = w.ground.get(fb, Holder::Program(prog as u32)).expect("the family remembers");
    assert!(m.holding(d.add(365)).is_some_and(|x| x.words.contains("pulled")));
    w.today = d.add(1);
    assert_eq!(stores::family_name(&w, b), Some(-1.0), "a family that holds it reads you at the bottom");
    assert!(stores::family_trust(&w, b).unwrap() <= 0.15 + 1e-9);
}

#[test]
fn word_travels_to_his_ties_at_half_and_his_area_at_a_quarter() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let area = w.high_schools[h].ground_area;
    let other = (0..w.high_schools.len())
        .find(|&x| x != h && w.high_schools[x].ground_area == area && w.high_schools[x].coach.is_some());
    let d = Date::ymd(2037, 5, 20);
    standing::day(&mut w, d);
    let who = hc(&w, h);
    let ph = Holder::Program(prog as u32);
    word::spread(&mut w, prog, who, 12.0, true, true, Some("Reyes".into()), d);
    for k in 0..15 {
        word::day(&mut w, d.add(k));
    }
    assert!(w.ground.travel.is_empty(), "all of it landed");
    let counselor = Who::Counselor(h as u32);
    let c = w.ground.get(counselor, ph).expect("his counselor heard");
    assert!((c.points - 6.0).abs() < 1e-3, "half: {}", c.points);
    assert!(c.moments.iter().any(|m| m.words == "I heard what you did for Reyes."), "{:?}", c.moments);
    assert_eq!(c.contacts, 0, "word isn't a contact");
    if let Some(o) = other {
        let m = w.ground.get(hc(&w, o), ph).expect("the area heard a big one");
        assert!((m.points - 3.0).abs() < 1e-3, "a quarter: {}", m.points);
        assert!(m.heard.as_deref() == Some("he heard what you did for Reyes"));
    }
    // A small thing reaches only the area's people who know you.
    let n = w.ground.memories.len();
    word::spread(&mut w, prog, who, 5.0, true, false, None, d.add(20));
    let far = w.ground.travel.iter().filter(|t| !t.close).count();
    let known = (0..w.high_schools.len())
        .filter(|&x| x != h && w.high_schools[x].ground_area == area)
        .filter_map(|x| people::head_coach_of(&w, x))
        .filter(|c| w.ground.get(*c, ph).is_some())
        .count();
    assert!(far <= known, "{far} > {known}");
    assert!(w.ground.memories.len() >= n);
}

#[test]
fn a_coach_who_moves_keeps_your_standing_and_his_successor_starts_known() {
    let (mut w, prog, _, h) = world();
    clean(&mut w, prog);
    let d = Date::ymd(2038, 1, 21);
    standing::day(&mut w, d);
    let who = hc(&w, h);
    let Who::Coach(c) = who else { unreachable!() };
    let ph = Holder::Program(prog as u32);
    {
        let m = w.ground.entry(who, ph, || Memory::new(who, ph, 0.0, d.add(-800)));
        m.points = 60.0;
        m.contacts = 9;
        m.known_since = Some(d.add(-700));
    }
    w.ground.set_told(who, Level::Trusted as u8);
    jobs::day(&mut w, d);
    // He joins another college's staff; his school fills the job.
    let other = (0..w.programs.len()).find(|&p| p != prog).unwrap();
    crate::world::staff::leave(&mut w, c as usize, "hired by a college");
    let group = w.coaches[c as usize].came_up;
    crate::world::staff::hire(&mut w, c as usize, Employer::College(other), Role::PositionCoach(group));
    let e = Employer::HighSchool(h);
    let pr = crate::world::staff::employer_prestige(&w, e);
    crate::world::staff::fill_now(&mut w, e, Role::HeadCoach, pr);
    jobs::day(&mut w, d.add(1));
    // What you built goes with him.
    assert_eq!(standing::points(&w, who, ph), 60.0);
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.iter().any(|m| m.subject.ends_with("moves on") && m.body[0].contains("goes with him")));
    assert!(people::role(&w, who).contains(w.program_name(other)));
    // The new man heard about you from him.
    let new = hc(&w, h);
    assert_ne!(new, who);
    assert_eq!(standing::level(&w, prog, new), Level::Known, "{}", standing::why(&w, prog, new));
    assert!(standing::why(&w, prog, new).contains("told him about you"));
    assert!(w.ground.vacated.is_empty());
}

#[test]
fn a_counselor_serves_a_tenure_and_the_next_one_starts_over() {
    let (w, _, _, h) = world();
    let (from, k) = people::counselor(&w, h as u32, 2037);
    assert!(from <= 2037 && from > 2037 - 15);
    let (from2, k2) = people::counselor(&w, h as u32, from);
    assert_eq!((from, k), (from2, k2), "the same tenure");
    // Some year within 15 the next one comes.
    let next = (2038..2053).find(|&y| people::counselor(&w, h as u32, y).1 != k).expect("a new counselor");
    assert_eq!(people::counselor(&w, h as u32, next).0, next);
    let name = people::name(&w, Who::Counselor(h as u32));
    assert!(name.starts_with("Ms. ") || name.starts_with("Mr. "), "{name}");
}

#[test]
fn your_networks_ground_people_read_through_the_memory() {
    let (mut w, prog, you, h) = world();
    clean(&mut w, prog);
    let who = hc(&w, h);
    let Who::Coach(c) = who else { unreachable!() };
    let today = w.today;
    standing::day(&mut w, today);
    w.folk.network.contacts.retain(|x| x.coach_id() != Some(c as usize));
    w.folk.network.contacts.push(crate::world::folk::network::Contact {
        coach: c as usize,
        warmth: 0.4,
        met: today.add(-900),
        how: "came up in coaching with you".into(),
        last: Some(today),
        recent: Vec::new(),
        balance: 2,
        other: None,
        sent: 0,
        record: (3, 2),
    });
    let k = w.folk.network.contacts.len() - 1;
    assert!(stores::ground_who(&w, &w.folk.network.contacts[k]).is_some());
    stores::sync(&mut w);
    let yh = Holder::Coach(you as u32);
    let m = w.ground.get(who, yh).unwrap();
    assert!(
        (m.points - 40.0).abs() < 1e-3 && m.owes == 2 && m.his_word == (3, 2),
        "{} {} {:?}",
        m.points,
        m.owes,
        m.his_word
    );
    assert!(m.roots.network);
    // The network's own work folds in: a favor asked, a warmer tie.
    w.folk.network.contacts[k].warmth += 0.05;
    w.folk.network.contacts[k].shift(-1);
    stores::sync(&mut w);
    let m = w.ground.get(who, yh).unwrap();
    assert!((m.points - 45.0).abs() < 1e-3, "{}", m.points);
    assert_eq!(m.owes, 1);
    assert_eq!(m.favor_words().as_deref(), Some("he owes you one"));
    // A visit by you moves the contact too: the memory is the truth.
    standing::contact(&mut w, prog, Some(you), who, Contact::Visit, 1.0, today);
    stores::sync(&mut w);
    let pts = standing::points(&w, who, yh);
    assert!((w.folk.network.contacts[k].warmth - pts / 100.0).abs() < 1e-5);
    assert!(pts > 45.0);
    // The network's monthly cooling leaves him to the memory.
    let before = w.folk.network.contacts[k].warmth;
    w.folk.network.contacts[k].last = Some(today.add(-200));
    crate::world::folk::network::day(&mut w, Date::ymd(2037, 9, 1));
    assert_eq!(w.folk.network.contacts[k].warmth, before);
}

#[test]
fn the_high_school_coachs_credit_lives_in_the_memory_and_an_old_file_moves_it() {
    let mut w = super::legacy::seat("hs");
    let p = 3usize;
    w.scouting.hs.credibility.insert(p, 0.8);
    w.scouting.hs.credibility.insert(5, 0.3);
    crate::world::ground::after_load(&mut w);
    assert!(w.scouting.hs.credibility.is_empty(), "moved into the memory");
    assert_eq!(crate::world::scouting::hs::credibility(&w, p), 0.8f32 as f64);
    assert_eq!(stores::credits(&w), vec![(3, 0.8), (5, 0.3)]);
    stores::set_credit(&mut w, p, 0.55);
    assert_eq!(stores::credit(&w, p), Some(0.55));
    let you = w.career.as_ref().unwrap().coach as u32;
    assert!(w.ground.get(Who::Coach(you), Holder::Program(p as u32)).is_some());
}

#[test]
fn a_persons_page_is_words_and_his_new_level_comes_in_his_own_words() {
    let (mut w, prog, you, h) = world();
    clean(&mut w, prog);
    let k = kids(&mut w, prog, h, 1);
    let d0 = w.today;
    standing::day(&mut w, d0);
    let who = hc(&w, h);
    let m = man(&w, prog, you);
    for k in 0..30 {
        standing::contact(&mut w, prog, Some(m), who, Contact::Visit, 1.0, d0.add(k));
    }
    let page = card::card(&w, prog, who);
    assert_eq!(page.level, "Known");
    assert!(page.why.starts_with("Known: "), "{}", page.why);
    assert!(!page.moments.is_empty(), "his Known letter is a moment he keeps");
    assert!(page.players.contains(&w.people[k[0]].player.id));
    assert!(page.hour.ends_with("PM"));
    let json = serde_json::to_string(&page).unwrap();
    for hidden in ["\"eye\"", "\"straight\"", "\"loyal\"", "\"warmth\"", "\"points\"", "\"grudge\""] {
        assert!(!json.contains(hidden), "a hidden number on the page: {hidden}");
    }
    // A talk with him reads the same memory.
    let lines = card::talk_lines(
        &w,
        crate::world::folk::persona::Them::Coach(match who {
            Who::Coach(c) => c as usize,
            _ => unreachable!(),
        }),
    );
    assert!(!lines.is_empty());
    // A family's new level in its own words.
    let fam = people::family_of(&w, k[0]);
    let id = w.people[k[0]].player.id;
    for d in 0..30 {
        crate::world::scouting::dept::add(&mut w, id, readers::COACH + m as u32, Kind::Call, d0.add(40 + 7 * d));
    }
    let _ = fam;
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.iter().any(|x| x.body.iter().any(|b| b.contains("We're listening"))), "the family's Known letter");
    assert!(w.ground.ups.iter().any(|u| u.1 == fam), "the morning map hears it");
}

#[test]
fn your_tree_and_your_old_players_are_ground_on_day_one() {
    let (mut w, prog, you, _) = world();
    // A coach who learned under you, coaching a high school.
    let c =
        (0..w.coaches.len()).find(|&c| c != you && people::coach_school(&w, c).is_some()).expect("a high school coach");
    w.coaches[c].mentor = Some(you);
    w.ground = Default::default();
    let today = w.today;
    standing::day(&mut w, today);
    let lv = standing::level(&w, prog, Who::Coach(c as u32));
    assert!(lv >= Level::Trusted, "{lv:?}");
    assert!(standing::why(&w, prog, Who::Coach(c as u32)).contains("your coaching tree"));
}
