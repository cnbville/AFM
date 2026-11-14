//! Discipline and boosters on the facts (talk facts lane L8, items 14, 17
//! and 27; `world/folk/rules.rs`, `arrests.rs`, `backers.rs`): he speaks
//! before you rule and each option says how it sits against his record; an
//! arrest and its court day come to you; every booster want can be kept or
//! broken by what you do, and breaking it costs; the four options that did
//! nothing now do something small.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::player::{DevelopmentEvent, PlayingTime, Trait};
use crate::world::folk::arrests::{self, Outcome};
use crate::world::folk::backers;
use crate::world::folk::cast::{self, CastRef, Role, Want};
use crate::world::folk::courts::{self, Case, Step};
use crate::world::folk::rules::{self, Response, RuleKind, Violation};
use crate::world::folk::scene::{self, Kind, PickKind, Scene};
use crate::world::folk::talks::{self, PromiseKind, PromiseTo};
use crate::world::folk::{Team, boss, press};
use crate::world::{Stage, World};

/// Your college program with `n` level-headed sophomore backups on it.
fn room(n: usize) -> (World, usize, Vec<usize>) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let v: Vec<usize> = (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(n).collect();
    for &i in &v {
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.trust = 0.5;
        q.mood = 0.0;
        q.bg.stability = 10.0;
        for t in Trait::ALL {
            q.player.personality.traits.set(t, 10.0);
        }
        w.programs[p].roster.push(i);
    }
    (w, p, v)
}

fn id(w: &World, i: usize) -> u32 {
    w.people[i].player.id
}

fn text_of(s: &Scene) -> String {
    s.lines.iter().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n")
}

/// A team-rule break waiting on you, as `rules::your_week` files it.
fn rule_break(w: &mut World, i: usize, kind: RuleKind, prior: u8) -> Scene {
    let who = id(w, i);
    let mut s = Scene::new(Kind::Discipline, w.today, "A break");
    s.a = who as u64;
    s.b = RuleKind::ALL.iter().position(|x| *x == kind).unwrap() as u64;
    s.c = prior as i64;
    s.stops = false;
    s.team = Team::seat(w);
    let sid = scene::add(w, s);
    w.folk.cases.push(Violation { id: who, kind, date: w.today, scene: sid, prior });
    scene::open(w, sid).unwrap()
}

/// The arrest scene `courts::amateur` files for the next morning, opened.
fn arrested(w: &mut World, i: usize, what: &str) -> Scene {
    let before = w.folk.scenes.len();
    courts::amateur(w, i, what, 1);
    let s = w.folk.scenes[before..]
        .iter()
        .find(|s| s.kind == Kind::Discipline && s.b == arrests::ARREST)
        .cloned()
        .expect("the arrest comes to you");
    assert_eq!(s.date, w.today.add(1), "you rule the next morning");
    assert!(s.stops, "an arrest stops the day");
    w.today = s.date;
    scene::open(w, s.id).unwrap()
}

#[test]
fn he_speaks_before_you_rule_and_the_options_read_his_record() {
    let (mut w, _, v) = room(2);
    let s = rule_break(&mut w, v[0], RuleKind::Curfew, 0);
    let text = text_of(&s);
    assert!(text.contains("Your staff's standard for it:"), "{text}");
    // He says his piece before the options, in a line of the bank or the
    // plain fallback.
    let his = s.lines.iter().filter(|l| l.by == 2).count();
    assert_eq!(his, 1, "he speaks once before you rule: {text}");
    assert!(!text.contains(['{', '}']), "{text}");
    // Every option says how it sits against the offense and his record.
    let fits = ["Fits what he did", "Heavy for", "Light for", "Far past", "Far under"];
    for p in &s.picks {
        assert!(fits.iter().any(|f| p.hint.contains(f)), "{}: {}", p.label, p.hint);
    }
    let dismiss = s.picks.iter().find(|p| p.code == 505).unwrap();
    assert!(dismiss.hint.contains("Far past"), "a first curfew: {}", dismiss.hint);
    let talk = s.picks.iter().find(|p| p.code == 500).unwrap();
    assert!(talk.hint.contains("Fits"), "{}", talk.hint);
    // A third break: a talk is light now.
    let s = rule_break(&mut w, v[1], RuleKind::Curfew, 2);
    let talk = s.picks.iter().find(|p| p.code == 500).unwrap();
    assert!(talk.hint.contains("Light") || talk.hint.contains("Far under"), "{}", talk.hint);
}

#[test]
fn an_arrest_comes_to_you_and_its_court_day_comes_back() {
    let (mut w, _, v) = room(3);
    let who = id(&w, v[0]);
    let s = arrested(&mut w, v[0], "a DUI");
    let text = text_of(&s);
    assert!(text.contains("a DUI") && text.contains("Your staff's standard"), "{text}");
    assert!(s.lines.iter().any(|l| l.by == 2), "he speaks first: {text}");
    let codes: Vec<u32> = s.picks.iter().map(|p| p.code).collect();
    assert_eq!(codes, vec![520, 521, 523, 524], "college options");
    for p in &s.picks {
        assert_eq!(p.kind == PickKind::Final, p.code != 523, "{}: a ruling can't be taken back", p.label);
    }
    // Suspended indefinitely: out until the court is done with it.
    let s = scene::answer(&mut w, s.id, 520).unwrap();
    assert!(s.done);
    assert!(w.folk.out.iter().any(|x| x.0 == who && x.2.starts_with("suspended indefinitely")));
    assert!(rules::sitting(&w, who, w.today.add(120)));
    let r = w.folk.rules.arrests.last().unwrap();
    assert_eq!((r.id, r.response, r.court), (who, Response::Indefinite, false));
    // The court day: the case closes, the outcome comes back to you.
    let case = w.folk.courts.open.iter().position(|c| c.id == who).unwrap();
    let opened = w.folk.courts.open[case].opened;
    w.folk.courts.open[case].step = Step::Court;
    w.folk.courts.open[case].guilty = false;
    w.folk.courts.open[case].next = w.today;
    let today = w.today;
    courts::day(&mut w, today);
    let s = w
        .folk
        .scenes
        .iter()
        .find(|s| s.b == arrests::COURT && s.a == who as u64)
        .cloned()
        .expect("the court day comes back to you");
    let s = scene::open(&mut w, s.id).unwrap();
    let text = text_of(&s);
    assert!(text.contains("case is over"), "{text}");
    let codes: Vec<u32> = s.picks.iter().map(|p| p.code).collect();
    assert_eq!(codes, vec![530, 531], "a cleared man out indefinitely: reinstate him or not");
    // Keeping a cleared man out reads as far past what he did.
    assert!(s.picks[1].hint.contains("Far past"), "{}", s.picks[1].hint);
    let mood = w.people[v[0]].mood;
    let s = scene::answer(&mut w, s.id, 530).unwrap();
    assert!(s.done);
    assert!(!w.folk.out.iter().any(|x| x.0 == who), "reinstated");
    assert!(w.people[v[0]].mood > mood, "relief");
    assert!(w.folk.rules.arrests.iter().any(|r| r.id == who && r.court && r.case == opened));
}

#[test]
fn a_guilty_man_who_played_owes_his_games_on_the_court_day() {
    let (mut w, _, v) = room(2);
    let who = id(&w, v[0]);
    let s = arrested(&mut w, v[0], "an arrest after a fight");
    let s = scene::answer(&mut w, s.id, 523).unwrap();
    assert!(s.done);
    assert!(!rules::sitting(&w, who, w.today.add(1)), "he plays");
    let case = w.folk.courts.open.iter().position(|c| c.id == who).unwrap();
    let opened = w.folk.courts.open[case].opened;
    let c = w.folk.courts.open.remove(case);
    w.folk.courts.closed.push(Case { step: Step::Closed, outcome: Some("pleaded".into()), ..c });
    arrests::settled(&mut w, who, opened, Outcome::Pleaded);
    let s = w.folk.scenes.iter().find(|s| s.b == arrests::COURT).cloned().unwrap();
    let s = scene::open(&mut w, s.id).unwrap();
    let codes: Vec<u32> = s.picks.iter().map(|p| p.code).collect();
    assert_eq!(codes, vec![537, 535, 536]);
    // A fight is two games: letting him play on is light, two more fits.
    assert!(s.picks[0].hint.contains("Far under") || s.picks[0].hint.contains("Light"), "{}", s.picks[0].hint);
    assert!(s.picks[1].label.contains("2 more games"), "{}", s.picks[1].label);
    assert!(s.picks[1].hint.contains("Fits"), "{}", s.picks[1].hint);
    let s = scene::answer(&mut w, s.id, 535).unwrap();
    assert!(s.done);
    assert!(w.folk.out.iter().any(|x| x.0 == who && x.2.starts_with("suspended 2 games")), "{:?}", w.folk.out);
}

#[test]
fn a_man_who_didnt_do_it_takes_the_same_ruling_harder() {
    let (mut w, _, v) = room(2);
    let mut after = Vec::new();
    for (k, guilty) in [(0, true), (1, false)] {
        let s = arrested(&mut w, v[k], "a DUI");
        let who = id(&w, v[k]);
        w.folk.courts.open.iter_mut().find(|c| c.id == who).unwrap().guilty = guilty;
        let before = w.people[v[k]].trust;
        scene::answer(&mut w, s.id, 520).unwrap();
        after.push(w.people[v[k]].trust - before);
    }
    assert!(after[1] < after[0], "the innocent man's trust falls further: {after:?}");
}

#[test]
fn a_ruling_you_hand_off_follows_your_staffs_standard() {
    let (mut w, _, v) = room(2);
    let s = arrested(&mut w, v[0], "an arrest after a fight");
    let s = scene::hand(&mut w, s.id).unwrap();
    assert!(s.done);
    let r = w.folk.rules.arrests.last().unwrap();
    assert_eq!(r.response, Response::Indefinite, "violence: indefinitely");
    let s = arrested(&mut w, v[1], "a DUI");
    scene::hand(&mut w, s.id).unwrap();
    assert_eq!(w.folk.rules.arrests.last().unwrap().response, Response::BenchGame, "a DUI: a game");
}

// ---------------------------------------------------------------------------
// Boosters (item 17).

fn booster(w: &World) -> cast::Cast {
    let team = Team::seat(w).unwrap();
    assert!(cast::has(w, team, Role::Booster));
    cast::get(w, CastRef::new(team, Role::Booster, 0))
}

fn promise(w: &mut World, want: Want) -> usize {
    let b = booster(w);
    talks::promise(w, PromiseTo::Cast(b.r, b.term), PromiseKind::Booster(want), 2);
    w.folk.promises.len() - 1
}

#[test]
fn every_booster_want_can_be_broken() {
    // A clean program: an arrest since the promise breaks it.
    let (mut w, _, v) = room(2);
    let k = promise(&mut w, Want::Clean);
    assert!(talks::kept(&w, &w.folk.promises[k].clone()));
    courts::amateur(&mut w, v[0], "a DUI", 1);
    assert!(!talks::kept(&w, &w.folk.promises[k].clone()), "an arrest since the promise");

    // A say on a player: his kid gets no look.
    let (mut w, _, v) = room(2);
    let k = promise(&mut w, Want::SayOnPlayer);
    let b = booster(&w);
    let kid = id(&w, v[0]);
    backers::promised_kid(&mut w, b.key(), kid);
    assert!(!talks::kept(&w, &w.folk.promises[k].clone()), "a backup with no games");
    w.people[v[0]].last_role = Some(PlayingTime::Rotation);
    assert!(talks::kept(&w, &w.folk.promises[k].clone()), "a real role");

    // A say in hires: a coordinator he never met.
    let (mut w, _, _) = room(1);
    let k = promise(&mut w, Want::SayInHires);
    let boss_key = boss::boss(&w, Team::seat(&w).unwrap()).key();
    let trust = w.folk.mine.trust_of(boss_key).unwrap_or(0.5);
    backers::hired(&mut w, 0, true);
    assert_eq!(w.folk.promises[k].kept, Some(false));
    assert!(w.folk.mine.trust_of(boss_key).unwrap_or(0.5) < trust, "he takes it to your boss");
    // ... and one he met first keeps it.
    let k = promise(&mut w, Want::SayInHires);
    let mut s = Scene::new(Kind::Interview, w.today, "An interview");
    backers::meet(&mut w, &mut s, 1);
    backers::hired(&mut w, 1, true);
    assert_eq!(w.folk.promises[k].kept, Some(true));

    // Access: you pull his pass.
    let (mut w, _, _) = room(1);
    let k = promise(&mut w, Want::Access);
    let team = Team::seat(&w).unwrap();
    let b = booster(&w);
    let mut s = Scene::new(Kind::Booster, w.today, "In the locker room");
    s.b = backers::PASS_TEST;
    backers::pass_open(&mut w, &mut s, team, &b);
    assert_eq!(s.picks.iter().map(|p| p.code).collect::<Vec<_>>(), vec![140, 141, 142]);
    backers::pass_answer(&mut w, &mut s, team, &b, 140);
    assert_eq!(w.folk.promises[k].kept, Some(false));

    // His name on a building: you leave it to the AD.
    let (mut w, _, _) = room(1);
    let k = promise(&mut w, Want::Building);
    let team = Team::seat(&w).unwrap();
    let mut s = Scene::new(Kind::BossMeeting, w.today, "A meeting");
    assert!(backers::naming_picks(&w, &mut s));
    backers::naming_answer(&mut w, &mut s, team, backers::NAMING + 1);
    assert_eq!(w.folk.promises[k].kept, Some(false));

    // His old school: its best prospect never heard from you.
    let (mut w, p, _) = room(1);
    let k = promise(&mut w, Want::OldSchool);
    if let Some(hs) = booster(&w).booster.and_then(|x| x.old_school) {
        let senior = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool && w.people[i].hs == hs);
        if let Some(senior) = senior {
            for q in w.people.iter_mut().filter(|q| q.hs == hs) {
                q.stars = 0;
            }
            w.people[senior].stars = 3;
            w.people[senior].class_of = w.year + 1;
            assert!(!talks::kept(&w, &w.folk.promises[k].clone()), "his school's best never heard from you");
            w.people[senior].cal.offers.push(p);
            assert!(talks::kept(&w, &w.folk.promises[k].clone()));
        }
    }
}

#[test]
fn a_broken_booster_promise_costs_more_than_his_trust() {
    let (mut w, p, _) = room(1);
    let k = promise(&mut w, Want::Clean);
    let team = Team::seat(&w).unwrap();
    let b = booster(&w);
    w.programs[p].collective.budget = 1.0e6;
    let boss_key = boss::boss(&w, team).key();
    let (bt, ct) = (w.folk.mine.trust_of(boss_key).unwrap_or(0.5), w.folk.mine.trust_of(b.key()).unwrap_or(0.5));
    let due = w.folk.promises[k].due;
    w.folk.rules.arrests.push(arrests::ArrestRuling {
        id: 1,
        case: w.today,
        charge: "a DUI".into(),
        response: Response::Wait,
        date: w.today,
        star: false,
        court: false,
    });
    w.today = due;
    talks::day(&mut w, due);
    assert_eq!(w.folk.promises[k].kept, Some(false));
    assert!(w.folk.mine.trust_of(b.key()).unwrap() < ct, "his trust");
    assert!(w.folk.mine.trust_of(boss_key).unwrap() < bt, "your boss hears");
    if b.booster.as_ref().is_some_and(|x| x.yearly > 0.0) {
        assert!(w.programs[p].collective.budget < 1.0e6, "he gives less");
    }
}

// ---------------------------------------------------------------------------
// The four options that did nothing (item 27).

#[test]
fn the_four_options_do_something_small() {
    // The fence: in-state recruits, for a year.
    let (mut w, p, _) = room(1);
    let st = w.programs[p].state;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    w.people[i].home_state = st;
    assert_eq!(press::fence_pull(&w, &w.people[i], p), 0.0);
    w.folk.fence = Some((st, w.today));
    assert_eq!(press::fence_pull(&w, &w.people[i], p), press::FENCE_PULL);
    assert_eq!(press::fence_pull(&w, &w.people[i], p + 1), 0.0, "only your program");
    w.people[i].home_state = (st + 1) % w.states.len();
    assert_eq!(press::fence_pull(&w, &w.people[i], p), 0.0, "only your state");
    w.people[i].home_state = st;
    w.folk.fence = Some((st, w.today.add(-press::FENCE_DAYS - 1)));
    assert_eq!(press::fence_pull(&w, &w.people[i], p), 0.0, "a year");

    // Calls to commits: a commit who heard from you holds on more.
    let year = w.year;
    w.programs[p].hc_changed = year;
    w.people[i].cal.commit = Some((p, w.today, false));
    let cold = crate::world::recruiting::commitment_hold(&w, i);
    w.folk.commit_calls = Some((p, w.today, true));
    w.folk.commits_heard = vec![(id(&w, i), true)];
    let you = crate::world::recruiting::commitment_hold(&w, i);
    w.folk.commits_heard = vec![(id(&w, i), false)];
    let staff = crate::world::recruiting::commitment_hold(&w, i);
    assert!(you > staff && staff > cold, "{cold} {staff} {you}");
    assert!((you / cold - 1.6).abs() < 1e-9, "0.8 of a normal year instead of half");
}

#[test]
fn pushed_rehab_brings_players_back_sooner_at_a_risk() {
    let (mut w, _, v) = room(2);
    for &i in &v {
        let mut ev = crate::player::Events::default();
        ev.add(DevelopmentEvent::MajorInjury);
        if let Some(h) = w.people[i].player.history.last_mut() {
            h.events = ev;
        }
    }
    if w.people[v[0]].player.history.is_empty() {
        return;
    }
    let today = Date(w.dates.spring_practice.0.0 - 30);
    w.today = today;
    let team = Team::seat(&w).unwrap();
    let pushed = crate::world::folk::takeover::rehab_holds(&mut w, team, true);
    assert_eq!(pushed, 2);
    let until: Vec<Date> = v.iter().map(|&i| w.training.hurt_on(id(&w, i), today).unwrap().until).collect();
    let rushed = w.folk.rushed.len();
    w.training.hurt.clear();
    w.folk.rushed.clear();
    let held = crate::world::folk::takeover::rehab_holds(&mut w, team, false);
    assert_eq!(held, 0);
    for (k, &i) in v.iter().enumerate() {
        let normal = w.training.hurt_on(id(&w, i), today).unwrap().until;
        assert!(until[k] < normal, "pushed back sooner");
    }
    assert_eq!(rushed, 2, "on the trainers' watch");
    let _ = Level::College;
}

// ---------------------------------------------------------------------------
// The scenes for review: `AFM_L8_DUMP=file.json cargo test --release
// scenes_for_review -- --ignored` writes each new scene as the screen gets
// it, for the mock-up.

#[test]
#[ignore]
fn scenes_for_review() {
    let mut out: Vec<(String, Scene)> = Vec::new();
    let (mut w, _, v) = room(4);
    out.push(("A curfew break, first time".into(), rule_break(&mut w, v[0], RuleKind::Curfew, 0)));
    out.push(("A drug test, third break".into(), rule_break(&mut w, v[1], RuleKind::DrugTests, 2)));
    let s = arrested(&mut w, v[2], "a DUI");
    out.push(("Arrest: the next morning".into(), s.clone()));
    scene::answer(&mut w, s.id, 520).unwrap();
    let who = id(&w, v[2]);
    let case = w.folk.courts.open.iter().position(|c| c.id == who).unwrap();
    w.folk.courts.open[case].step = Step::Court;
    w.folk.courts.open[case].guilty = false;
    w.folk.courts.open[case].next = w.today;
    let today = w.today;
    courts::day(&mut w, today);
    let c = w.folk.scenes.iter().find(|s| s.b == arrests::COURT && s.a == who as u64).cloned().unwrap();
    out.push(("Court day: cleared, suspended indefinitely".into(), scene::open(&mut w, c.id).unwrap()));
    let s = arrested(&mut w, v[3], "an arrest after a fight");
    out.push(("Arrest after a fight".into(), s.clone()));
    scene::answer(&mut w, s.id, 523).unwrap();
    let who = id(&w, v[3]);
    let case = w.folk.courts.open.iter().position(|c| c.id == who).unwrap();
    w.folk.courts.open[case].step = Step::Court;
    w.folk.courts.open[case].guilty = true;
    w.folk.courts.open[case].next = w.today;
    let today = w.today;
    courts::day(&mut w, today);
    let c = w.folk.scenes.iter().find(|s| s.b == arrests::COURT && s.a == who as u64).cloned().unwrap();
    out.push(("Court day: guilty, he played".into(), scene::open(&mut w, c.id).unwrap()));

    let (mut w, _, _) = room(1);
    promise(&mut w, Want::Access);
    let team = Team::seat(&w).unwrap();
    let b = booster(&w);
    let mut s = Scene::new(Kind::Booster, w.today, format!("{} in the locker room", b.name));
    s.b = backers::PASS_TEST;
    backers::pass_open(&mut w, &mut s, team, &b);
    out.push(("Booster: in the locker room".into(), s));
    promise(&mut w, Want::Building);
    let mut s = Scene::new(Kind::BossMeeting, w.today, "The facility");
    backers::naming_picks(&w, &mut s);
    out.push(("The AD: whose name goes on it".into(), s));
    promise(&mut w, Want::SayInHires);
    if let Some((code, label)) = backers::meet_pick(&w, 0, true) {
        let mut s = Scene::new(Kind::Interview, w.today, "An interview");
        s.picks.push(scene::Pick {
            label,
            code,
            hint: "Keeps your word to the booster. A proud candidate notices.".into(),
            ..Default::default()
        });
        out.push(("Interview: the lunch option".into(), s));
    }
    if let Ok(path) = std::env::var("AFM_L8_DUMP") {
        std::fs::write(path, serde_json::to_string_pretty(&out).unwrap()).unwrap();
    }
}
