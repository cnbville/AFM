//! Ground lane G4: news in order, leads, finds (`world/ground/news.rs`,
//! `leads.rs`, `finds.rs`).

use crate::calendar::Date;
use crate::world::World;
use crate::world::ground::leads::{self, Blank, How, Lead};
use crate::world::ground::news::{self, What, Why};
use crate::world::ground::standing::{self, Level};
use crate::world::ground::{Holder, Memory, Moment, Who, finds, people};
use crate::world::scouting::{Kind, readers};

/// The tiny college seat, with no day-one ground anywhere, a school with a
/// head coach, and three college players sent back to it as recruits.
fn world() -> (World, usize, usize, Vec<u32>) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    for p in &mut w.programs {
        p.hs_pipeline.clear();
        p.area_pipeline.clear();
        p.footprint = [0.0; 51];
        p.pipeline = [0.0; 51];
    }
    for c in &mut w.coaches {
        c.ties.clear();
    }
    let h = (0..w.high_schools.len()).find(|&h| w.high_schools[h].coach.is_some()).expect("a school with a coach");
    let season = w.dates.season;
    let picks: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].college != Some(prog)).take(3).collect();
    let mut kids = Vec::new();
    for &i in &picks {
        let q = &mut w.people[i];
        q.stage = crate::world::Stage::HighSchool;
        q.college = None;
        q.class_of = season + 1;
        q.hs = h;
        q.stars = 2;
        q.cal.signed = None;
        q.cal.commit = None;
        q.cal.offers.clear();
        q.cal.early_offers.clear();
        q.player.age = q.player.age.clamp(16, 17);
        kids.push(q.player.id);
    }
    (w, prog, h, kids)
}

fn coach(w: &World, h: usize) -> Who {
    people::head_coach_of(w, h).unwrap()
}

fn idx(w: &World, id: u32) -> usize {
    crate::world::scouting::place(w, id).unwrap()
}

/// Your program's memory with a person at a level's points, met long ago.
fn stand(w: &mut World, prog: usize, who: Who, points: f32) {
    let d = w.today;
    let m =
        w.ground.entry(who, Holder::Program(prog as u32), || Memory::new(who, Holder::Program(prog as u32), 0.0, d));
    m.points = points;
    m.contacts = 6;
    m.met = d.add(-6 * 365);
    m.known_since = Some(d.add(-4 * 365));
}

/// Two other FBS programs.
fn rivals(w: &World, prog: usize) -> (usize, usize) {
    let v: Vec<usize> = (0..w.programs.len())
        .filter(|&j| {
            j != prog
                && w.programs[j].division == crate::world::data::Division::FBS
                && w.programs[j].head_coach.is_some()
        })
        .take(2)
        .collect();
    (v[0], v[1])
}

#[test]
fn news_goes_in_order_of_standing_then_his_nature() {
    let (mut w, prog, h, _) = world();
    let who = coach(&w, h);
    let (a, b) = rivals(&w, prog);
    // A powerhouse school: its coach is busy and answers in order.
    w.high_schools[h].prestige = 0.8;
    w.high_schools[h].national = false;
    // Both rivals Home with him (the school is their pipeline), you Trusted.
    w.programs[a].hs_pipeline.insert(h, (0.95, 2030));
    w.programs[b].hs_pipeline.insert(h, (1.0, 2030));
    w.programs[a].prestige = 0.95;
    w.programs[b].prestige = 0.3;
    stand(&mut w, prog, who, 60.0);
    assert_eq!(Level::of(standing::derived(&w, who, Holder::Program(a as u32))), Level::Home);
    let d = w.today;
    let Who::Coach(c) = who else { unreachable!() };
    // Loyal: the staff he's closest to first (b has done more for him).
    w.coaches[c as usize].personality.loyalty = 20.0;
    let o = news::order(&w, What::Grew(20), Some(who), h, d, 1, None);
    let pos = |o: &news::Order, j: usize| o.heard.iter().position(|x| x.prog as usize == j);
    println!("loyal: {:?}", o.heard);
    assert!(pos(&o, b) < pos(&o, a), "a loyal coach calls the staff he's closest to");
    assert_eq!(o.heard[0].why, Why::Closest);
    assert_eq!(o.place, 3, "Home before Trusted: you hear third");
    assert_eq!(o.you.level, Level::Trusted as u8);
    assert!(o.you.day >= o.heard[1].day && o.you.day < o.public);
    // Self-serving: the biggest program first.
    w.coaches[c as usize].personality.loyalty = 1.0;
    let o = news::order(&w, What::Grew(20), Some(who), h, d, 1, None);
    println!("self-serving: {:?}", o.heard);
    assert!(pos(&o, a) < pos(&o, b), "a self-serving coach calls the biggest program");
    assert_eq!(o.heard[0].why, Why::Biggest);
    // A small school's coach calls everyone he knows at once.
    w.high_schools[h].prestige = 0.2;
    let o = news::order(&w, What::Grew(20), Some(who), h, d, 1, None);
    assert!(o.heard.iter().all(|x| x.why == Why::Pushes && x.day.0 - d.0 <= 1), "{:?}", o.heard);
    // A coach who holds a broken word stops calling you.
    let m = w.ground.get_mut(who, Holder::Program(prog as u32)).unwrap();
    m.keep(Moment {
        date: d,
        words: "You pulled my kid's offer.".into(),
        good: false,
        big: true,
        holds_to: Some(d.add(900)),
    });
    let o = news::order(&w, What::Grew(20), Some(who), h, d, 1, None);
    assert_eq!(o.place, 0);
    assert_eq!(o.you.why, Why::Stopped);
    assert!(o.you.day > o.public, "you hear after the public");
    // The paper reaches everyone at once.
    let o = news::order(&w, What::Track(1068), None, h, d, 1, None);
    assert!(o.heard.is_empty());
    assert_eq!(o.you.why, Why::Paper);
    assert_eq!(o.you.day, d);
    // The words name the order.
    let kid = 1;
    let n = news::make(&w, kid, What::Offer(a as u32), "An offer.".into(), d, Some(who), h, None, 0);
    let words = news::order_words(&w, &n);
    println!("{words} / {}", news::why_words(&w, &n));
    assert!(words.contains("You:"), "{words}");
}

/// The coach's read of his kids is placed in a class of many weaker kids.
fn class_of_weak_kids(w: &mut World, kids: &[u32]) {
    let season = w.dates.season;
    let h = w.people[idx(w, kids[0])].hs;
    let mut by = Vec::new();
    for k in 1..=3 {
        by.push((season + k, vec![-1.0e9; 1000]));
    }
    let list: Vec<usize> = kids.iter().map(|&id| idx(w, id)).collect();
    let mut map = std::collections::HashMap::new();
    map.insert(h, list);
    w.ground.classes = leads::Classes { day: Some(w.today), by, kids: map };
}

#[test]
fn his_coach_gives_his_list_by_your_standing() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    class_of_weak_kids(&mut w, &kids);
    let d = w.today;
    // A Stranger gets the public answer a day or two later.
    let i0 = idx(&w, kids[0]);
    w.people[i0].stars = 3;
    leads::coach_list(&mut w, prog, None, h, d);
    let l = leads::lead(&w, kids[0]).expect("the name everyone has");
    assert!(l.since > d && l.since <= d.add(2), "a day or two later");
    assert_eq!(l.level, Level::Stranger as u8);
    assert!(leads::lead(&w, kids[1]).is_none(), "a stranger doesn't get his real list");
    // Again within the month: nothing new.
    stand(&mut w, prog, who, 30.0);
    leads::coach_list(&mut w, prog, None, h, d.add(5));
    assert!(leads::lead(&w, kids[1]).is_none(), "once a month a school");
    // Known: his real list, the same day, in his own words.
    leads::coach_list(&mut w, prog, None, h, d.add(31));
    let l = leads::lead(&w, kids[1]).expect("on his real list");
    assert_eq!(l.since, d.add(31));
    assert_eq!(l.how, How::List);
    assert!(l.claim.is_some() && l.height.is_some() && l.forty.is_some());
    println!("Known: {}", l.said);
    assert!(l.said.contains(&w.people[idx(&w, kids[1])].name));
    assert!(!w.scouting.dept.looks.contains_key(&kids[1]), "no truth at Known");
    // Trusted: the truth about the kid and his tape before you ask.
    stand(&mut w, prog, who, 60.0);
    leads::drop(&mut w, kids[2]).ok();
    leads::coach_list(&mut w, prog, None, h, d.add(62));
    let l = leads::lead(&w, kids[2]).expect("Trusted list");
    println!("Trusted: {}", l.said);
    assert!(w.scouting.dept.looks.get(&kids[2]).is_some_and(|v| v.iter().any(|x| x.kind == Kind::Call)));
    assert!(w.scouting.dept.asks.iter().any(|a| a.0 == kids[2] && a.1 == Kind::HsFilm));
    let b = leads::blanks(&w, kids[2]);
    assert!(b.iter().any(|f| f.blank == Blank::Character && f.filled.is_some()));
}

#[test]
fn news_reaches_you_on_its_day_as_a_lead_and_a_call() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    stand(&mut w, prog, who, 80.0);
    w.high_schools[h].prestige = 0.8;
    let d = w.today;
    let name = w.people[idx(&w, kids[0])].name.clone();
    let n = news::make(&w, kids[0], What::Grew(25), format!("{name} grew 2.5 inches."), d, Some(who), h, None, 1);
    assert_eq!(n.place, 1, "Home and alone: he calls you first");
    let day = n.you.day;
    assert!(news::keep(&mut w, n.clone()));
    assert!(!news::keep(&mut w, n), "kept once");
    news::deliver(&mut w, prog, day.add(-1));
    if day > d {
        assert!(leads::lead(&w, kids[0]).is_none(), "not before its day");
    }
    news::deliver(&mut w, prog, day);
    let l = leads::lead(&w, kids[0]).expect("a lead");
    assert_eq!(l.how, How::Called);
    assert!(l.said.contains("first"), "{}", l.said);
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.iter().any(|m| m.subject == format!("About {name}")), "his call at his hour");
    // A four-star already rated by the public doesn't make a new lead.
    let i1 = idx(&w, kids[1]);
    w.people[i1].stars = 4;
    let n = news::make(&w, kids[1], What::Grew(25), "Grew.".into(), d, Some(who), h, None, 2);
    news::keep(&mut w, n);
    news::deliver(&mut w, prog, d.add(10));
    assert!(leads::lead(&w, kids[1]).is_none());
}

#[test]
fn a_big_game_and_a_laser_time_and_an_offer_make_news() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    stand(&mut w, prog, who, 30.0);
    let d = w.today;
    let i0 = idx(&w, kids[0]);
    let mut s = [0i32; crate::world::hs_year::HS_STAT_COUNT];
    s[crate::world::hs_year::HsStat::RushYds as usize] = 240;
    s[crate::world::hs_year::HsStat::RushTd as usize] = 3;
    let other = (h + 1) % w.high_schools.len();
    news::on_hs_game(&mut w, d, [h, other], &[vec![(i0, s, true)], Vec::new()]);
    let n = w.ground.news.iter().find(|n| n.kid == kids[0] && n.what == What::BigGame).expect("a big game");
    println!("{}", n.line);
    assert!(n.line.contains("ran for 240 yards"));
    // A laser time at another program's camp, and that program's offer.
    let (a, _) = rivals(&w, prog);
    let monday = d.on_or_after(crate::calendar::Weekday::Mon).add(7);
    let ran = monday.add(-3);
    let i1 = idx(&w, kids[1]);
    w.people[i1].player.position = crate::positions::Position::WR;
    w.times.by_id.entry(kids[1]).or_default().push(crate::world::times::Time {
        event: crate::world::times::Event::Forty,
        timed: crate::world::times::Timed::Laser,
        secs: 4.41,
        date: ran,
        at: crate::world::times::At::Camp(a as u16),
        lb: 180,
    });
    let i2 = idx(&w, kids[2]);
    w.people[i2].cal.early_offers.push((a, ran));
    news::week(&mut w, prog, monday);
    let laser = w.ground.news.iter().find(|n| n.kid == kids[1]).expect("a laser time");
    assert!(matches!(laser.what, What::Laser(441, Some(_))));
    assert_eq!(
        laser.order.first().map(|x| (x.prog as usize, x.why)),
        Some((a, Why::There)),
        "the camp's own program first"
    );
    let offer = w.ground.news.iter().find(|n| n.kid == kids[2]).expect("an offer");
    assert_eq!(offer.what, What::Offer(a as u32));
    // The week again: nothing twice.
    let n = w.ground.news.len();
    news::week(&mut w, prog, monday);
    assert_eq!(w.ground.news.len(), n);
}

#[test]
fn a_kid_the_world_rates_low_is_your_find_and_his_blooms_pay() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    stand(&mut w, prog, who, 60.0);
    class_of_weak_kids(&mut w, &kids);
    let d = w.today;
    leads::coach_list(&mut w, prog, None, h, d);
    assert!(leads::lead(&w, kids[0]).is_some());
    // A four-star you offer isn't a find; a two-star is.
    let i1 = idx(&w, kids[1]);
    w.people[i1].stars = 4;
    finds::on_offer(&mut w, prog, kids[1], d);
    assert!(w.ground.finds.is_empty());
    finds::on_offer(&mut w, prog, kids[0], d);
    let f = w.ground.finds[0].clone();
    assert_eq!(f.teller, Some(who), "credited to the one who told you");
    assert!((f.pay - 1.0).abs() < 1e-6, "a two-star with no offers pays in full");
    assert_eq!(finds::pay(3, 3, 5), 0.4);
    let men_before = f.man.map(|c| finds::man(&w, c).finds);
    assert!(men_before.is_none_or(|n| n == 1));
    // A fourth star: a small bloom.
    let i0 = idx(&w, kids[0]);
    w.people[i0].stars = 4;
    let before = standing::points(&w, who, Holder::Program(prog as u32));
    finds::day(&mut w, prog, d.add(1));
    let f = &w.ground.finds[0];
    assert_eq!(f.steps, 1, "{:?}", f.blooms);
    assert!(w.ground.finder > 0.0);
    assert!(standing::points(&w, who, Holder::Program(prog as u32)) > before);
    assert_eq!(leads::record(&w, who), (1, 1), "his kid held up");
    let m = w.ground.get(who, Holder::Program(prog as u32)).unwrap();
    assert!(m.moments.iter().any(|x| x.words.contains("You were first")), "{:?}", m.moments);
    // All-conference: a bloom.
    w.stats.honors.insert(
        kids[0],
        crate::world::records::Honors {
            name: "x".into(),
            pos: None,
            all_america: Vec::new(),
            all_conference: 1,
            awards: Vec::new(),
            inducted: None,
        },
    );
    finds::day(&mut w, prog, d.add(2));
    assert_eq!(w.ground.finds[0].steps, 2);
    // He signs elsewhere: still your find.
    let (a, _) = rivals(&w, prog);
    w.people[i0].cal.signed = Some(crate::world::Signed {
        program: a,
        walk_on: false,
        date: d.add(3),
        early_enrollee: false,
        flipped: true,
        committed: None,
    });
    finds::day(&mut w, prog, d.add(3));
    let f = &w.ground.finds[0];
    assert_eq!(f.lost.map(|x| x.1 as usize), Some(a));
    assert_eq!(f.setback.as_ref().map(|x| x.1), Some(finds::Setback::Flip));
    assert_eq!(f.steps, 2, "he's still your find");
}

#[test]
fn an_oversold_kid_drops_the_tellers_record() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    let d = w.today;
    let i0 = idx(&w, kids[0]);
    let tall = crate::body::body_now(&w.people[i0].player).height as f32 + 2.5;
    leads::add(
        &mut w,
        Lead {
            kid: kids[0],
            since: d,
            how: How::List,
            from: Some(who),
            tipper: None,
            man: None,
            level: 1,
            said: "\"He's 6'4\".\"".into(),
            height: Some(tall),
            forty: None,
            claim: Some(1),
            news: Vec::new(),
            word: None,
            dropped: false,
        },
    );
    let man = w.programs[prog].head_coach.unwrap();
    crate::world::scouting::dept::add(&mut w, kids[0], readers::COACH + man as u32, Kind::Camp, d);
    assert_eq!(leads::record(&w, who), (0, 1));
    assert_eq!(leads::lead(&w, kids[0]).unwrap().word, Some(false));
    assert!(leads::record_words(&w, who).contains("0 of his 1"));
    let mails = &w.career.as_ref().unwrap().desk.mails;
    assert!(mails.iter().any(|m| m.subject.contains("oversold")));
    // Your camp filled two blanks and the live look.
    let b = leads::blanks(&w, kids[0]);
    for k in [Blank::Size, Blank::Speed, Blank::Live] {
        assert!(b.iter().any(|f| f.blank == k && f.filled.is_some()), "{k:?}");
    }
}

#[test]
fn a_blank_fills_by_its_look_and_your_board_reads_your_ground() {
    let (mut w, prog, h, kids) = world();
    let d = w.today;
    let i0 = idx(&w, kids[0]);
    w.people[i0].stars = 1;
    // Not a lead, no stars, not yours: off your board.
    let board = crate::world::scouting::dept::watch(&w, prog);
    assert!(!board.contains(&i0));
    leads::add(
        &mut w,
        Lead {
            kid: kids[0],
            since: d,
            how: How::Tape,
            from: None,
            tipper: None,
            man: None,
            level: 0,
            said: String::new(),
            height: None,
            forty: None,
            claim: None,
            news: Vec::new(),
            word: None,
            dropped: false,
        },
    );
    let board = crate::world::scouting::dept::watch(&w, prog);
    assert!(board.contains(&i0), "a lead is on your board");
    // Filling the tape blank asks for his film; once watched it's filled.
    let t = leads::fill(&mut w, kids[0], Blank::Tape).unwrap();
    println!("{t}");
    assert!(w.scouting.dept.asks.iter().any(|a| a.0 == kids[0] && a.1 == Kind::HsFilm));
    assert!(leads::fill(&mut w, kids[0], Blank::Tape).is_err());
    crate::world::scouting::dept::add(&mut w, kids[0], readers::PERSONNEL, Kind::HsFilm, d);
    assert!(leads::fill(&mut w, kids[0], Blank::Tape).is_err(), "filled already");
    // Size: he's invited to your camp in June, and he comes.
    leads::fill(&mut w, kids[0], Blank::Size).unwrap();
    let season = w.dates.season;
    let camp = Date::nth_weekday(season, 6, crate::calendar::Weekday::Sat, 2);
    w.today = camp;
    crate::world::scouting::dept::day(&mut w, camp);
    assert!(leads::blanks(&w, kids[0]).iter().any(|f| f.blank == Blank::Size && f.filled.is_some()), "he came");
    assert!(!w.scouting.dept.asks.iter().any(|a| a.0 == kids[0] && a.1 == Kind::Camp));
    // The test switch puts the built board back.
    w.ground.off = true;
    assert!(leads::Board::of(&w, prog).is_none());
    let _ = h;
}

#[test]
fn the_new_store_saves_and_an_old_file_loads_empty() {
    let (mut w, prog, h, kids) = world();
    let who = coach(&w, h);
    stand(&mut w, prog, who, 80.0);
    let d = w.today;
    let n = news::make(&w, kids[0], What::Grew(25), "Grew.".into(), d, Some(who), h, None, 1);
    news::keep(&mut w, n);
    news::deliver(&mut w, prog, d.add(5));
    finds::on_offer(&mut w, prog, kids[0], d);
    let bytes = rmp_serde::to_vec_named(&w.ground).unwrap();
    let back: crate::world::ground::Ground = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back.news.len(), w.ground.news.len());
    assert_eq!(back.leads.len(), w.ground.leads.len());
    assert_eq!(back.finds.len(), 1);
    let old: crate::world::ground::Ground = serde_json::from_str("{\"memories\": []}").unwrap();
    assert!(old.leads.is_empty() && old.news.is_empty() && old.finds.is_empty());
    println!("ground store: {} news, {} leads, {} bytes", w.ground.news.len(), w.ground.leads.len(), bytes.len());
}
