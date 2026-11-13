//! One-on-ones that read his facts (talk facts lane L4, item 6;
//! `world/folk/talk_points.rs`): his talking points are the chips, an
//! arrested man's talk opens on the arrest every time, each point offers
//! its own moves in his stance, a benched man asks what it takes, a move
//! counts by what it rests on, the game keeps what you talked through, an
//! arrest moves his mood outside the talk, getting him help lowers his
//! risk, and the AI coaches' talks read the same way.

use super::legacy::seat;
use crate::calendar::{Date, Level};
use crate::player::{PlayingTime, Trait};
use crate::world::folk::claims::{self, Kind as K, sheet};
use crate::world::folk::courts::{self, Case, Step};
use crate::world::folk::life::{Event, Happening};
use crate::world::folk::scene::{self, PickKind, Scene};
use crate::world::folk::talk_points::{self as tp, Answer, Stance, Topic};
use crate::world::folk::talks::{self, Move, PromiseKind, PromiseTo, Subject};
use crate::world::folk::voice::Facts;
use crate::world::folk::{Team, rules};
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
        q.mood_why = 0;
        q.cal.lean = 0.0;
        q.bg.gpa = 3.4;
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

fn arrest(w: &mut World, i: usize, guilty: bool) {
    let today = w.today;
    let who = id(w, i);
    w.folk.courts.open.push(Case {
        id: who,
        level: Level::College,
        what: "a DUI arrest".into(),
        opened: today.add(-2),
        next: today.add(19),
        step: Step::Arrest,
        guilty,
        outcome: None,
    });
}

/// Opens a talk on a topic, with a fresh week of hours.
fn talk(w: &mut World, who: u32, topic: Topic) -> Scene {
    w.folk.mine.hours = 0.0;
    let sid = talks::start_talk_on(w, who, topic).unwrap();
    scene::open(w, sid).unwrap()
}

fn pick(s: &Scene, mv: Move) -> u32 {
    s.picks
        .iter()
        .find(|p| p.mv == mv.name())
        .unwrap_or_else(|| panic!("{} on the table: {:?}", mv.name(), s.picks))
        .code
}

#[test]
fn an_arrested_man_opens_on_the_arrest_every_time() {
    let (mut w, _, v) = room(2);
    arrest(&mut w, v[0], true);
    let who = id(&w, v[0]);
    // The arrest is a chip in its own words, and his behavior is a subject
    // only when something is behind it.
    let chips = talks::subjects_for(&w, who);
    assert!(chips.iter().any(|c| *c == ("arrest", "The DUI arrest".to_string())), "{chips:?}");
    assert!(chips.iter().any(|c| c.0 == "behavior"));
    let clean = talks::subjects_for(&w, id(&w, v[1]));
    assert!(!clean.iter().any(|c| c.0 == "behavior" || c.0 == "arrest"), "{clean:?}");
    let book = tp::book_says(&w, who, Level::College);
    // Your staff names it in plain words.
    let sh = claims::player(&w, v[0]);
    assert_eq!(tp::worry_words(sh.get(K::Arrest).unwrap()), "the DUI arrest is all he can think about");
    for n in 0..3 {
        // You call him in about his role; he opens on the arrest.
        let s = talk(&mut w, who, Subject::Role.into());
        assert_eq!(Topic::of(s.b), Topic::Point(K::Arrest), "talk {n}");
        assert!(Stance::from_bits(s.c).is_some());
        for l in &s.lines {
            assert!(!l.text.contains(['{', '}']), "{}", l.text);
        }
        let moves: Vec<&str> = s.picks.iter().filter(|p| p.code != 900).map(|p| p.mv.as_str()).collect();
        assert!((3..=5).contains(&moves.len()), "{moves:?}");
        assert!(s.picks.iter().filter(|p| p.code != 900).all(|p| p.point == "arrest"));
        let promise = s.picks.iter().find(|p| p.mv == "Promise").unwrap();
        assert_eq!(promise.kind, PickKind::Final, "a promise can't be taken back");
        let straight = s.picks.iter().find(|p| p.mv.starts_with("Tell")).unwrap();
        assert!(straight.label.contains(&book), "{} / {book}", straight.label);
        assert_eq!(s.picks.iter().find(|p| p.mv == "Warn").unwrap().hint, tp::HINT_WARN);
        let s = scene::answer(&mut w, s.id, pick(&s, Move::Ask)).unwrap();
        let s = scene::answer(&mut w, s.id, 900).unwrap();
        assert!(s.done);
        let sh = claims::player(&w, v[0]);
        let p = sh.get(K::Arrest).unwrap();
        assert_eq!(w.folk.talked.times(w.today, who, p), n as u8 + 1, "what you talked through is kept");
        w.today = w.today.add(2);
    }
}

#[test]
fn his_stance_reads_his_traits_and_the_truth() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    arrest(&mut w, i, true);
    let count = |w: &World, s: Stance| {
        let sh = claims::player(w, i);
        let p = sh.get(K::Arrest).unwrap().clone();
        (0..300u64).filter(|&k| tp::stance(w, i, &p, k * 7919) == s).count()
    };
    // A professional who trusts you owns what he did; a hothead with an
    // ego who doesn't, fights it.
    w.people[i].trust = 0.9;
    w.people[i].player.personality.traits.set(Trait::Professionalism, 18.0);
    let owns = count(&w, Stance::Owns);
    assert!(owns > 150, "a professional owns it: {owns} of 300");
    w.people[i].trust = 0.1;
    for t in [Trait::Ego, Trait::Temper] {
        w.people[i].player.personality.traits.set(t, 18.0);
    }
    w.people[i].player.personality.traits.set(Trait::Professionalism, 3.0);
    let owns = count(&w, Stance::Owns);
    let fights = count(&w, Stance::Defiant) + count(&w, Stance::Denies) + count(&w, Stance::PlaysDown);
    assert!(owns < 20 && fights > 200, "a hothead fights it: owns {owns}, fights {fights}");
    // A man who didn't do it denies it, whoever he is.
    w.folk.courts.open[0].guilty = false;
    let denies = count(&w, Stance::Denies);
    assert!(denies > 200, "the innocent deny it: {denies}");

    // What a line may claim: an open case backs nothing, unless his stance
    // makes it true to say.
    let sh = claims::player(&w, i);
    let p = sh.get(K::Arrest).unwrap();
    let facts = |st: Stance| {
        let mut f = Facts::new();
        tp::conduct(&sh, Some(p), Some(st), &mut f);
        (f.get("blameless").is_some(), f.get("at_fault").is_some())
    };
    assert_eq!(facts(Stance::Denies), (true, false), "he didn't do it and says so");
    assert_eq!(facts(Stance::Owns), (false, false), "owning what he didn't do admits nothing");
    w.folk.courts.open[0].guilty = true;
    let sh = claims::player(&w, i);
    let p = sh.get(K::Arrest).unwrap();
    let mut f = Facts::new();
    tp::conduct(&sh, Some(p), Some(Stance::Denies), &mut f);
    assert!(f.get("blameless").is_none() && f.get("at_fault").is_none(), "a guilty man's denial claims nothing");
    let mut f = Facts::new();
    tp::conduct(&sh, Some(p), Some(Stance::Owns), &mut f);
    assert_eq!(f.get("at_fault"), Some("yes"), "he did it and owns it");

    // A guilty man never says he didn't do it, in any talk.
    for n in 0..40 {
        let who = id(&w, i);
        let s = talk(&mut w, who, Topic::Point(K::Arrest));
        for l in s.lines.iter().filter(|l| l.by == 2) {
            let t = l.text.to_lowercase();
            assert!(!t.contains("i didn't do it") && !t.contains("i did nothing wrong"), "talk {n}: {}", l.text);
        }
        scene::answer(&mut w, s.id, 900).unwrap();
        w.folk.scenes.clear();
    }
}

#[test]
fn a_move_counts_by_what_it_rests_on() {
    let (mut w, _, v) = room(2);
    let (a, b) = (v[0], v[1]);
    arrest(&mut w, a, true);
    let sh = claims::player(&w, a);
    let topic = Topic::Point(K::Arrest);
    // A warning on his arrest rests on a fact on his mind.
    let r = tp::rest(&w, a, &sh, topic, Move::Warn, false);
    assert!(!r.against && (r.k - sheet::ON_MIND).abs() < 1e-6, "{r:?}");
    // While the case is open only he knows whether he did it, so a warning
    // is never against the facts (and nothing on screen says he didn't).
    w.folk.courts.open[0].guilty = false;
    let sh = claims::player(&w, a);
    assert!(!tp::rest(&w, a, &sh, topic, Move::Warn, false).against);
    // Once the charges are dropped, warning him is against the facts: the
    // lift is gone and it costs trust.
    let mut c = w.folk.courts.open.remove(0);
    c.next = w.today.add(-3);
    c.outcome = Some("The charges were dropped.".into());
    w.folk.courts.closed.push(c);
    let sh = claims::player(&w, a);
    assert!(sh.get(K::CaseClosed).is_some(), "the closed case is on his sheet");
    let topic_closed = Topic::Point(K::CaseClosed);
    let r = tp::rest(&w, a, &sh, topic_closed, Move::Warn, false);
    assert!(r.against && (0.03..=0.06).contains(&r.cost), "{r:?}");
    let fair = tp::rest(&w, a, &sh, topic_closed, Move::Ask, false);
    let (dt_against, _) =
        talks::landing(&w, a, Move::Warn, crate::world::folk::takeover::Tone::Firm, Subject::Behavior, r, 1);
    let (dt_fair, _) = talks::landing(
        &w,
        a,
        Move::Warn,
        crate::world::folk::takeover::Tone::Firm,
        Subject::Behavior,
        tp::Rest { against: false, cost: 0.0, ..fair },
        1,
    );
    assert!(dt_against < dt_fair - 0.02, "{dt_against} against {dt_fair}");
    // A warning about behavior to a clean man is against the facts too.
    let clean = claims::player(&w, b);
    assert!(tp::rest(&w, b, &clean, Subject::Behavior.into(), Move::Warn, false).against);
    // Praise on a subject he isn't worried about is the plain version.
    let r = tp::rest(&w, b, &clean, Subject::Role.into(), Move::Praise, false);
    assert!((r.k - sheet::PLAIN).abs() < 1e-6, "{r:?}");

    // Made once and landed, it counts half as much more the next time.
    w.folk.courts.closed.clear();
    arrest(&mut w, a, true);
    let sh = claims::player(&w, a);
    let p = sh.get(K::Arrest).unwrap().clone();
    w.folk.talked.note(w.today, id(&w, a), &p, true, "straight");
    w.today = w.today.add(1);
    let sh = claims::player(&w, a);
    let r = tp::rest(&w, a, &sh, topic, Move::Straight, false);
    let lift = sh.basis(K::Arrest).lift();
    let want = sheet::PLAIN + (lift - sheet::PLAIN) * 0.5;
    assert!((r.k - want).abs() < 1e-5, "{} wanted {want}", r.k);
}

#[test]
fn a_benched_man_asks_what_it_takes() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    // Last year's starter, benched this week.
    w.people[i].last_role = Some(PlayingTime::Starter);
    talks::mood_event(&mut w, who, talks::Reason::Benched, -0.1);
    let sh = claims::player(&w, i);
    assert!(sh.get(K::LostJob).is_some(), "the bench he felt is a lost job");
    let s = talk(&mut w, who, Topic::Point(K::LostJob));
    assert_eq!(Topic::of(s.b), Topic::Point(K::LostJob));
    // He asks; your answers are on the table, and a promise is final.
    let codes: Vec<u32> = s.picks.iter().map(|p| p.code).collect();
    for a in Answer::ALL {
        assert!(codes.contains(&a.code()), "{a:?} in {codes:?}");
    }
    assert!(s.picks.iter().find(|p| p.code == Answer::Promise.code()).is_some_and(|p| p.kind == PickKind::Final));
    let asked = s.lines.iter().rev().find(|l| l.by == 2).unwrap().text.clone();
    assert!(!asked.is_empty() && !asked.contains('{'), "{asked}");
    // You promise: the job back is a start, logged, though he started last
    // year; then the point's moves, with no second promise.
    let label = s.picks.iter().find(|p| p.code == Answer::Promise.code()).unwrap().label.clone();
    assert!(label.contains("the job is yours"), "{label}");
    let s = scene::answer(&mut w, s.id, Answer::Promise.code()).unwrap();
    let made = w.folk.promises.iter().find(|p| matches!(p.to, PromiseTo::Player(t) if t == who)).expect("logged");
    assert_eq!(made.kind, PromiseKind::Start);
    assert!(s.picks.iter().any(|p| p.point == "lost_job" && p.mv == "Challenge"), "{:?}", s.picks);
    assert!(!s.picks.iter().any(|p| p.mv == "Promise"), "a promise lands once: {:?}", s.picks);
    // Back on the point later in the talk, he doesn't ask again.
    let sid = s.id;
    talks::change_topic(&mut w, sid, Subject::Team.into()).unwrap();
    talks::change_topic(&mut w, sid, Topic::Point(K::LostJob)).unwrap();
    let s = w.folk.scenes.iter().find(|x| x.id == sid).unwrap();
    assert!(!s.picks.iter().any(|p| (200..240).contains(&p.code)), "asked once: {:?}", s.picks);
    assert!(s.f[0] != 0.0 || s.f[1] != 0.0, "the answer moved him");
    // A dodge costs a little.
    let (dt, dm) = tp::answer_landing(&w, i, Answer::Dodge).unwrap();
    assert!(dt < 0.0 && dm < 0.0);
}

#[test]
fn praise_for_the_same_award_twice_and_he_says_so() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    let season = w.dates.season;
    w.stats.honors.entry(who).or_default().awards.push((season, "Biletnikoff Award"));
    w.today = w.dates.awards_show.add(2);
    let sh = claims::player(&w, i);
    let p = sh.get(K::Award).expect("the award is on his sheet").clone();
    let r1 = tp::rest(&w, i, &sh, Topic::Point(K::Award), Move::Praise, false);
    assert_eq!(r1.truth, Some(1.0), "praise for an award is true");
    let s = talk(&mut w, who, Topic::Point(K::Award));
    // Good news with no opening of its own takes the good news part, never
    // a subject's ("Is this about where I'm at on the depth chart?").
    let bank = crate::world::folk::voice::bank();
    let opened = s.lines.iter().find(|l| l.by == 2).unwrap().text.clone();
    let good: Vec<String> = ["humble", "proud"]
        .iter()
        .flat_map(|st| bank.versions(&format!("talk.point.open.good.{st}")))
        .map(|x| x.0.to_string())
        .collect();
    assert!(good.contains(&opened), "{opened}");
    // An award is not a worry: an unhappy winner's reason is something else.
    w.people[i].mood = -0.3;
    assert!(!talks::concern(&w, i).1.contains("award"), "{}", talks::concern(&w, i).1);
    w.people[i].mood = 0.0;
    let praise = s.picks.iter().find(|x| x.mv == "Praise").unwrap();
    assert!(praise.label.contains("Biletnikoff"), "{}", praise.label);
    let s = scene::answer(&mut w, s.id, praise.code).unwrap();
    scene::answer(&mut w, s.id, 900).unwrap();
    let row = w.folk.talked.row(who, &p).expect("kept");
    assert_eq!(row.said, "praise");
    // The next day: it counts less, and he says he heard it.
    w.today = w.today.add(1);
    let sh = claims::player(&w, i);
    let r2 = tp::rest(&w, i, &sh, Topic::Point(K::Award), Move::Praise, false);
    assert!(r2.k < r1.k, "{} then {}", r1.k, r2.k);
    let s = talk(&mut w, who, Topic::Point(K::Award));
    let code = s.picks.iter().find(|x| x.mv == "Praise").unwrap().code;
    let s = scene::answer(&mut w, s.id, code).unwrap();
    let said = s.lines.iter().rev().find(|l| l.by == 2).unwrap().text.clone();
    let again: Vec<String> = bank.versions("talk.point.again").iter().map(|x| x.0.to_string()).collect();
    assert!(again.contains(&said), "he says he heard it: {said}");
}

#[test]
fn trouble_at_home_he_tells_becomes_the_topic() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    let today = w.today;
    w.folk.life.events.push(Happening { id: who, event: Event::ParentIll, date: today.add(-3), told: false });
    // An older wedding kept after it: telling you one isn't telling the other.
    w.folk.life.events.push(Happening { id: who, event: Event::Wedding, date: today.add(-20), told: false });
    assert!(!claims::player(&w, i).known().any(|p| p.kind == K::SickParent), "you don't know yet");
    // He trusts you and opens up easily: he raises it himself.
    w.people[i].trust = 1.0;
    w.people[i].player.personality.traits.set(Trait::Adaptability, 20.0);
    w.people[i].player.personality.traits.set(Trait::Ego, 0.0);
    let s = talk(&mut w, who, Subject::Team.into());
    assert_eq!(Topic::of(s.b), Topic::Point(K::SickParent));
    let told = |e: Event| w.folk.life.events.iter().any(|x| x.event == e && x.told);
    assert!(told(Event::ParentIll), "you know now");
    assert!(!told(Event::Wedding), "the wedding he didn't tell");
    assert!(s.picks.iter().any(|p| p.mv.starts_with("Hand") && p.point == "sick_parent"));
    // Time away, as on the life subject.
    let code = s.picks.iter().find(|p| p.mv.starts_with("Hand")).unwrap().code;
    scene::answer(&mut w, s.id, code).unwrap();
    assert!(w.folk.out.iter().any(|x| x.0 == who), "he gets time away");
}

#[test]
fn an_arrest_moves_his_mood_and_help_lowers_his_risk() {
    let (mut w, _, v) = room(1);
    let i = v[0];
    let who = id(&w, i);
    let mood = w.people[i].mood;
    courts::amateur(&mut w, i, "a DUI", 1);
    assert!(w.folk.courts.open.iter().any(|c| c.id == who), "a case opens");
    let drop = mood - w.people[i].mood;
    assert!((0.09..=0.23).contains(&drop), "the arrest moved his mood by {drop}");
    let why = talks::reasons(&w, who);
    assert!(why.iter().any(|x| x.1.contains("his arrest")), "{why:?}");
    // You get him help: once a year, it doesn't stack.
    let s = talk(&mut w, who, Topic::Point(K::Arrest));
    let code = s.picks.iter().find(|p| p.mv.starts_with("Hand")).unwrap().code;
    let s = scene::answer(&mut w, s.id, code).unwrap();
    assert!(tp::helped(&w, who), "{:?}", s.outcome);
    assert!(s.outcome.iter().any(|x| x.contains("help")), "{:?}", s.outcome);
    assert!(!tp::help(&mut w, who), "it doesn't stack");
    assert!(tp::helped_ids(&w).contains(&who));
    w.today = w.today.add(tp::HELP_DAYS);
    assert!(!tp::helped(&w, who), "for a year");
    // Your staff rules the way the book says.
    assert_eq!(rules::staff_response(0, Level::College), rules::Response::Talk);
}

#[test]
fn a_talking_point_is_never_named_past_its_window() {
    let (mut w, p, _) = room(4);
    let start = w.today;
    let roster: Vec<usize> = Team::College(p).roster(&w).into_iter().take(25).collect();
    let mut seen = 0;
    for week in 0..53 {
        w.today = start.add(7 * week);
        for &i in &roster {
            let sh = claims::player(&w, i);
            for c in tp::chips(&sh) {
                seen += 1;
                assert!(c.known && tp::talkable(c.kind));
                if let Some(d) = c.day {
                    assert!(sheet::in_window(c.kind, d, w.today), "{:?} from {d:?} on {:?}", c.kind, w.today);
                }
            }
        }
    }
    assert!(seen > 0, "some point came up in a year");
}

#[test]
fn ai_coaches_talk_the_same_way() {
    // The test world is tiny: a class of sophomores spread over the other
    // programs, unhappy as the men an AI coach talks to, a third of them
    // just benched.
    let mut w = seat("college");
    let mine = w.seat.and_then(|s| s.program);
    w.add_class_aged(19);
    let year = w.year;
    let others: Vec<usize> = (0..w.programs.len()).filter(|&j| Some(j) != mine).collect();
    let players: Vec<usize> =
        (0..w.people.len()).filter(|&i| w.people[i].stage == Stage::HighSchool).take(240).collect();
    for (n, &i) in players.iter().enumerate() {
        let p = others[n % others.len()];
        let q = &mut w.people[i];
        q.stage = Stage::College;
        q.college = Some(p);
        q.class_of = year - 1;
        q.college_seasons = 1;
        q.last_role = Some(PlayingTime::Backup);
        q.mood = -0.35;
        w.programs[p].roster.push(i);
        if n % 3 == 0 {
            let who = id(&w, i);
            talks::mood_event(&mut w, who, talks::Reason::Benched, -0.1);
        }
    }
    eprintln!("AI talk sample: {} players", players.len());
    assert!(players.len() >= 100);
    let mean = |rel: f32| {
        let v: Vec<(f32, f32)> = players.iter().map(|&i| tp::ai_talk(&w, i, rel, 0xA17A ^ (i as u64) << 8)).collect();
        let n = v.len() as f32;
        (v.iter().map(|x| x.0).sum::<f32>() / n, v.iter().map(|x| x.1).sum::<f32>() / n)
    };
    let (good, mid, bad) = (mean(18.0), mean(10.0), mean(2.0));
    for x in [good.0, good.1, mid.0, mid.1, bad.0, bad.1] {
        assert!(x.is_finite() && x.abs() < 0.3);
    }
    // The old flat rule, for the report: good 0.25 + 0.035 x rel of the time.
    let old = |rel: f32| {
        let p = 0.25 + 0.035 * rel;
        (0.04 * p - 0.03 * (1.0 - p), 0.08 * p - 0.03 * (1.0 - p))
    };
    eprintln!(
        "AI talks, mean (trust, mood) per talk: rel 18 {good:?} (old {:?}); rel 10 {mid:?} (old {:?}); rel 2 {bad:?} (old {:?})",
        old(18.0),
        old(10.0),
        old(2.0)
    );
    assert!(
        good.0 + good.1 > mid.0 + mid.1 && mid.0 + mid.1 > bad.0 + bad.1,
        "a coach who reads people does better: {good:?} {mid:?} {bad:?}"
    );
}

#[test]
fn the_topic_survives_a_save() {
    for k in K::ALL.iter().copied().filter(|k| tp::talkable(*k)) {
        let t = Topic::Point(k);
        assert_eq!(Topic::of(t.code()), t, "{k:?}");
        assert_eq!(Topic::from_key(k.key()), Some(t));
        assert_ne!(Topic::of(t.code()).subject(), Subject::Friend);
    }
    for s in Subject::EVERY {
        assert_eq!(Topic::of(Topic::Subject(s).code()), Topic::Subject(s));
        assert_eq!(Topic::from_key(s.key()), Some(Topic::Subject(s)));
    }
    // A scene with no points made saves as before, and reads back.
    let mut s = Scene::new(scene::Kind::OneOnOne, Date::ymd(2026, 9, 1), "t");
    s.b = Topic::Subject(Subject::Money).code();
    let j = serde_json::to_string(&s).unwrap();
    assert!(!j.contains("made"), "{j}");
    let back: Scene = serde_json::from_str(&j).unwrap();
    assert!(back.made.is_empty());
    assert_eq!(Topic::of(back.b), Topic::Subject(Subject::Money));
}
