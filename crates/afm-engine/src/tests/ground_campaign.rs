//! Ground lane G6: the offseason campaign (`world/ground/campaign/`): the
//! goal list, your AD's talk after signing day, the February meeting, your
//! own goal and the hunt at a Monday meeting, flips tracked, the verdict
//! with its report, the class settling at signing day, and your first
//! offseason's guide.

use crate::calendar::{Date, Weekday};
use crate::world::folk::Team;
use crate::world::folk::boss::{self, GoalKind};
use crate::world::folk::scene::Kind;
use crate::world::ground::campaign::{self, Slot, Spring, goals};
use crate::world::{Stage, World};

/// The tiny college seat with a class to recruit: pros made into high
/// school seniors-to-be (the class of `class`) and juniors in your state.
fn college(class: i32) -> (World, usize) {
    let mut w = super::legacy::seat("college");
    let prog = w.seat.and_then(|s| s.program).unwrap();
    let st = w.programs[prog].state;
    let schools: Vec<usize> = (0..w.high_schools.len()).filter(|&h| w.high_schools[h].state == st).collect();
    assert!(!schools.is_empty(), "schools in your state");
    for (k, p) in w.people.iter_mut().filter(|p| p.stage == Stage::Pro).take(160).enumerate() {
        p.stage = Stage::HighSchool;
        p.pro = None;
        p.class_of = class + (k % 2) as i32;
        p.player.age = 17;
        p.hs = schools[k % schools.len()];
        p.home_state = st;
        p.stars = [2, 3, 3, 4, 5][k % 5];
        p.cal.commit = None;
        p.cal.signed = None;
        p.cal.offers.clear();
        p.cal.early_offers.clear();
    }
    (w, prog)
}

fn day(w: &mut World, d: Date) {
    w.today = d;
    campaign::day(w, d);
}

fn run(w: &mut World, from: Date, to: Date) {
    let mut d = from;
    while d <= to {
        day(w, d);
        d = d.add(1);
    }
}

fn kids(w: &World, class: i32) -> Vec<usize> {
    (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].class_of == class).collect()
}

#[test]
fn the_goal_list_has_its_63_in_order() {
    assert_eq!(goals::GOALS.len(), 63);
    for (k, d) in goals::GOALS.iter().enumerate() {
        assert_eq!(d.id as usize, k + 1, "{}", d.key);
        assert!((1..=3).contains(&d.classes), "{}", d.key);
        assert!(!d.raised.is_empty(), "{}", d.key);
        assert_eq!(goals::by_key(d.key).map(|x| x.id), Some(d.id), "keys are unique: {}", d.key);
    }
    // The scale (GR39): two-thirds Mostly, a third Partly.
    assert_eq!(goals::Verdict::of(1.0), goals::Verdict::Done);
    assert_eq!(goals::Verdict::of(2.0 / 3.0), goals::Verdict::Mostly);
    assert_eq!(goals::Verdict::of(0.5), goals::Verdict::Partly);
    assert_eq!(goals::Verdict::of(0.2), goals::Verdict::Missed);
    // Two of three state #1s is Mostly (GR38, GR39).
    assert_eq!(goals::Verdict::of(goals::share_over(&[1.0, 0.0, 1.0], 3)), goals::Verdict::Mostly);
    // A sure thing pays little and costs most; a long shot the other way (GR41).
    let (sure_gain, sure_loss) = campaign::worth(0.9);
    let (long_gain, long_loss) = campaign::worth(0.1);
    assert!(sure_gain < long_gain && sure_loss > long_loss);
}

#[test]
fn a_spring_runs_from_the_ads_talk_to_the_verdict() {
    let year = 2038;
    let class = year + 1;
    let (mut w, prog) = college(class);
    let team = Team::College(prog);
    let s = Spring::of(year);
    assert_eq!(s.meeting.weekday(), Weekday::Mon);
    assert!(s.meeting > s.signing);

    // Signing day: the AD's talk goes on the calendar for the next morning.
    day(&mut w, s.signing);
    let k = w
        .folk
        .scenes
        .iter()
        .position(|x| x.kind == Kind::BossMeeting && x.c == campaign::talk::CODE)
        .expect("his talk is set");
    assert_eq!(w.folk.scenes[k].date, s.talk);
    let mut sc = w.folk.scenes.remove(k);
    w.today = s.talk;
    boss::open(&mut w, &mut sc);
    assert!(!sc.docs.is_empty(), "his motives on the table");
    let codes: Vec<u32> = sc.picks.iter().map(|p| p.code).collect();
    for c in [60, 61, 62, 64] {
        assert!(codes.contains(&c), "pick {c} in {codes:?}");
    }
    let before = boss::trust(&w, team);
    // Ask why first (the goal stays on the table), then take each as set.
    let why = sc.picks.iter().find(|p| p.code == 64).unwrap().clone();
    boss::answer(&mut w, &mut sc, 64, &why);
    assert!(!sc.done && sc.picks.iter().all(|p| p.code != 64));
    while !sc.done {
        let take = sc.picks.iter().find(|p| p.code == 60).unwrap().clone();
        boss::answer(&mut w, &mut sc, 60, &take);
    }
    assert!(boss::trust(&w, team) > before, "agreeing builds a little trust");
    let ad: Vec<_> = w.ground.campaign.goals.iter().filter(|h| h.slot == Slot::Ad).cloned().collect();
    assert!(!ad.is_empty() && ad.len() <= 2, "one or two goals he holds you to (GR35)");
    assert!(ad.iter().all(|h| h.told.ad));
    let season = boss::goal_season(&w, team);
    assert!(campaign::ad_set(&w, season), "his goals are boss goals for the season");
    assert!(
        !boss::draft_goals(&w, team).iter().any(|g| matches!(g, GoalKind::Recruiting(_))),
        "the summer meeting leaves his class goal to the spring talk"
    );

    // The February meeting: needs, his word, a ground goal.
    run(&mut w, s.talk.add(1), s.meeting);
    let c = &w.ground.campaign;
    assert_eq!((c.year, c.class), (year, class));
    for slot in [Slot::Needs, Slot::Ad, Slot::Yours] {
        assert!(c.goals.iter().any(|h| h.slot == slot && h.open()), "{slot:?}");
    }
    assert!(!c.feb.is_empty(), "your ground as it stood in February");
    let memo = w.career.as_ref().unwrap().desk.mails.iter().any(|m| m.subject.starts_with("The February meeting"));
    assert!(memo, "the coordinator's letter");

    // Not a Monday: goals don't change.
    w.today = s.meeting.add(2);
    assert!(campaign::pick(&mut w, goals::Aim { goal: 5, n: 2, of: 0, class }).is_err());
    // The next Monday: your own goal (the coordinator's counts as dropped)
    // and the hunt.
    let monday = s.meeting.add(7);
    day(&mut w, monday);
    let said = campaign::pick(&mut w, goals::Aim { goal: 5, n: 2, of: 0, class }).unwrap();
    assert!(said.contains("dropped"), "{said}");
    campaign::hunt(&mut w, true).unwrap();
    let c = &w.ground.campaign;
    assert_eq!(c.goals.iter().filter(|h| h.slot == Slot::Yours && h.open()).count(), 1);
    assert!(c.goals.iter().any(|h| h.slot == Slot::Yours && h.dropped.is_some()));
    assert!(c.goals.iter().any(|h| h.slot == Slot::Hunt && h.open()));
    assert!(campaign::drop(&mut w, Slot::Ad).is_err(), "your AD's goals change only by talking to him");

    // A flip in: a kid committed elsewhere commits to you.
    let class_kids = kids(&w, class);
    let other = (0..w.programs.len()).find(|&j| j != prog).unwrap();
    let flip = class_kids[0];
    w.people[flip].cal.offers.push(prog);
    w.people[flip].cal.commit = Some((other, monday, false));
    day(&mut w, monday.add(1));
    w.people[flip].cal.commit = Some((prog, monday.add(2), false));
    day(&mut w, monday.add(2));
    assert_eq!(w.ground.campaign.flipped_in.len(), 1, "the flip is tracked");

    // The spring's commitments: two blue-chips, three more.
    let mut blue = 0;
    for &i in class_kids.iter().skip(1) {
        let p = &mut w.people[i];
        if p.stars < 4 || blue >= 2 {
            continue;
        }
        blue += 1;
        p.cal.commit = Some((prog, monday.add(30), false));
    }
    assert_eq!(blue, 2);
    let lines = campaign::letter_lines(&w, prog);
    assert!(lines.iter().any(|l| l.contains("blue-chips")), "{lines:?}");

    // The verdict.
    let v = campaign::verdict_day(&w, prog, year);
    assert_eq!(v.weekday(), Weekday::Mon);
    assert!(v.month() >= 8, "{}", v.pretty());
    let trust_before = boss::trust(&w, team);
    run(&mut w, monday.add(3), v);
    let c = &w.ground.campaign;
    assert_eq!(c.reports.len(), 1, "the report is kept");
    let r = &c.reports[0];
    assert_eq!(r.title, "Your first spring");
    let bc = r.goals.iter().find(|g| g.words.starts_with("2 blue-chips")).expect("your goal on the report");
    assert_eq!(bc.verdict, goals::Verdict::Done, "{bc:?}");
    assert!(r.goals.iter().any(|g| g.dropped), "the dropped goal shows");
    assert!(!r.ad.is_empty());
    let yours = c.goals.iter().find(|h| h.slot == Slot::Yours && h.open()).unwrap();
    assert_eq!(yours.parts[0].pace, Some(1.0));
    assert!((boss::trust(&w, team) - trust_before).abs() < 0.2, "a little each, never a firing on its own");

    // Signing day settles the class.
    let sd = campaign::signing_of(class);
    for &i in &class_kids {
        if w.people[i].cal.commit.is_some_and(|c| c.0 == prog) {
            w.people[i].cal.signed = Some(crate::world::Signed {
                program: prog,
                walk_on: false,
                date: sd,
                early_enrollee: false,
                flipped: false,
                committed: None,
            });
        }
    }
    day(&mut w, sd.add(1));
    let yours = w.ground.campaign.goals.iter().find(|h| h.slot == Slot::Yours && h.aim.goal == 5).unwrap();
    assert!(yours.settled, "{yours:?}");
    assert_eq!(yours.parts[0].result, Some(1.0));
}

#[test]
fn every_goal_reads_whole_words() {
    let year = 2038;
    let class = year + 1;
    let (mut w, prog) = college(class);
    w.today = Spring::of(year).meeting;
    for &i in kids(&w, class).iter().take(6) {
        w.people[i].cal.commit = Some((prog, w.today, false));
    }
    let kid = w.people[kids(&w, class)[0]].player.id;
    let area = goals::campus_area(&w, prog).unwrap_or(0) as u32;
    for d in goals::GOALS.iter() {
        let of = match d.subj {
            goals::Subj::Kid => kid,
            goals::Subj::Area => area,
            goals::Subj::Rival => goals::rival(&w, prog).unwrap_or((prog + 1) % w.programs.len()) as u32,
            goals::Subj::School => w.people[kids(&w, class)[0]].hs as u32,
            goals::Subj::State => w.programs[prog].state as u32,
            _ => 0,
        };
        let a = goals::Aim { goal: d.id, n: d.n.max(1), of, class };
        let r = goals::read(&w, prog, &a, class);
        assert!((0.0..=1.0).contains(&r.share), "{}: {r:?}", d.key);
        assert!(!r.words.contains('{') && !r.words.contains("NaN"), "{}: {r:?}", d.key);
        let words = goals::words(&w, &a);
        assert!(!words.is_empty() && !words.contains('{'), "{}: {words}", d.key);
        let p = goals::likely(&w, prog, &a, 0.3);
        assert!((0.05..=0.95).contains(&p), "{}: {p}", d.key);
    }
    // His motives read the world: the conference-class goal is always there.
    let ms = campaign::motives::motives(&w, prog, class);
    assert!(ms.iter().any(|m| m.aim.goal == 2), "{ms:?}");
    assert!(ms.iter().all(|m| !m.why.is_empty()));
}

#[test]
fn a_first_offseason_waits_for_the_window_then_your_coordinator_guides_you() {
    let year = 2038;
    let (mut w, prog) = college(year + 1);
    let s = Spring::of(year);
    let jan1 = Date::ymd(year, 1, 1);
    w.folk.mine.since = Some(jan1);
    if let Some(t) = w.folk.takeover.as_mut() {
        t.rush = true;
    } else {
        w.folk.takeover = Some(crate::world::folk::takeover::Takeover { rush: true, ..Default::default() });
    }
    assert!(campaign::waits(&w, jan1.add(5)), "the takeover and the portal first (GR31)");
    assert!(!campaign::waits(&w, s.door));
    run(&mut w, jan1, s.door);
    let g = &w.ground.campaign.guide;
    assert!(g.on && g.door == Some(s.door), "{g:?}");
    assert!(campaign::tool_open(&w, "map"));
    assert!(!campaign::tool_open(&w, "week"), "the week planner opens at the February meeting");
    assert!(w.career.as_ref().unwrap().desk.mails.iter().any(|m| m.subject == "Our ground"));
    run(&mut w, s.door.add(1), s.meeting);
    assert!(campaign::tool_open(&w, "week"));
    let lines = campaign::letter_lines(&w, prog);
    assert!(lines.iter().any(|l| l.starts_with("My read")), "{lines:?}");
    campaign::guide_off(&mut w).unwrap();
    assert!(campaign::TOOLS.iter().all(|t| campaign::tool_open(&w, t)));
}
