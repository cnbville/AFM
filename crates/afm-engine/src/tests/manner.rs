//! How you talk (`docs/design/coach-voice.md`, `persona::manner`): the
//! five dials and their words, the eight prototypes, the same line landing
//! differently on each man by what he wants from a coach, what he has with
//! you and where it was said, wear, a read that fades, the stores, the
//! typed line through the conversation, complaints to an outside review and
//! a firing for cause, and AI coaches on the same rules.

use super::legacy::seat;
use crate::player::PlayingTime;
use crate::world::folk::persona::Them;
use crate::world::folk::persona::ear::{Reading, Resolving};
use crate::world::folk::persona::manner::{
    self, Acc, Complaint, HALF_LIFE_DAYS, Manner, Prototype, Setting, TookAs, Wants, land,
};
use crate::world::folk::persona::talk::{self, Conversation, Parts, Said, channels, store};
use crate::world::folk::talks::{self, Subject};
use crate::world::{Stage, World};

fn line(heat: &str, language: &str, aim: &str, care: &str, say: &str) -> Manner {
    Manner::from_words(heat, language, aim, care, say)
}

/// "We're going to fucking destroy the SEC."
fn destroy() -> Manner {
    line("hot", "profane", "out", "cold", "tell")
}

/// "That was fucking soft. You embarrassed this whole room."
fn soft() -> Manner {
    line("hot", "profane", "man", "cold", "tell")
}

/// "I'm on you because you're better than this, and I'm not letting you
/// waste it."
fn on_you() -> Manner {
    line("firm", "clean", "work", "caring", "tell")
}

fn competitor() -> Wants {
    Wants { hard: 0.8, swears: true, ..Wants::default() }
}

fn shaky() -> Wants {
    Wants { hard: -0.6, shaky: 0.7, proud: 0.3, ..Wants::default() }
}

#[test]
fn the_ears_words_are_the_dials() {
    let m = soft();
    assert!(m.heat > 0.7 && m.profane > 0.8 && m.aim > 0.8 && m.care < 0.1 && m.ask < 0.2);
    assert_eq!(m.words(), "hot, profane, aimed at the man, cold, tell");
    // A word the ear doesn't have reads as the middle.
    assert_eq!(Manner::from_words("loud", "x", "y", "z", "w").heat, manner::HEAT[2].1);
}

#[test]
fn the_prototypes_name_where_the_dials_sit() {
    for p in Prototype::ALL {
        assert_eq!(Prototype::from_key(p.key()), Some(p));
        if p != Prototype::Showman {
            assert_eq!(manner::prototypes(&p.center(), None).0, p, "{p:?}");
        }
    }
    assert_eq!(manner::prototypes(&destroy(), None).0, Prototype::Firebreather);
    assert_eq!(manner::prototypes(&line("calm", "clean", "out", "caring", "ask"), None).0, Prototype::PlayersCoach);
    // The Showman is the public face, only when it runs hotter than the
    // private one.
    let calm = line("even", "clean", "out", "neutral", "tell");
    assert_ne!(manner::prototypes(&calm, None).0, Prototype::Showman);
    assert_eq!(manner::prototypes(&calm, Some(&Prototype::Showman.center())).0, Prototype::Showman);
    assert!(manner::words_for(&destroy(), None).starts_with("a Firebreather"));
}

#[test]
fn heat_aimed_out_reads_as_passion_to_almost_everyone() {
    let set = Setting::default();
    let c = land(&destroy(), &competitor(), 0.5, None, &set);
    let s = land(&destroy(), &shaky(), 0.5, None, &set);
    assert!(c.mood > 0.0 && s.mood >= 0.0, "{c:?} {s:?}");
    assert!(c.mood > s.mood, "the competitor feeds off it more");
    assert_eq!(c.as_, TookAs::FiredUp);
    assert_eq!(s.as_, TookAs::Flat);
    assert_eq!(c.abuse, 0.0);
}

#[test]
fn aimed_at_the_man_it_is_abuse_unless_the_relationship_carries_it() {
    let set = Setting::default();
    // No trust: it hurts, most of all the shaky.
    let cold = land(&soft(), &competitor(), 0.1, None, &set);
    let weak = land(&soft(), &shaky(), 0.1, None, &set);
    assert!(cold.trust < 0.0 && weak.trust < cold.trust, "{cold:?} {weak:?}");
    assert_eq!(weak.as_, TookAs::ShutDown);
    assert!(weak.abuse > 0.0);
    // Deep trust and a man who wants it hard: tough love.
    let deep = land(&soft(), &competitor(), 0.95, None, &set);
    assert!(deep.trust > cold.trust);
    assert!(deep.abuse == 0.0);
    // "I'm on you because you're better than this": care carries the aim.
    let care = land(&on_you(), &competitor(), 0.4, None, &set);
    assert!(care.trust > 0.0, "{care:?}");
}

#[test]
fn shown_up_in_public_stings_the_proud_professional_more() {
    let pro = Wants { hard: 0.2, proud: 0.6, pro: 0.6, swears: true, ..Wants::default() };
    let private = land(&soft(), &pro, 0.3, None, &Setting::default());
    let public = land(&soft(), &pro, 0.3, None, &Setting { public: true, ..Setting::default() });
    assert!(public.trust < private.trust, "{private:?} {public:?}");
    assert_eq!(public.as_, TookAs::PushesBack);
}

#[test]
fn cursing_in_front_of_his_family_lands_badly_and_the_nfl_is_used_to_it() {
    let home = Setting { family: true, ..Setting::default() };
    let clean = land(&line("hot", "clean", "out", "cold", "tell"), &competitor(), 0.5, None, &home);
    let cursed = land(&destroy(), &competitor(), 0.5, None, &home);
    assert!(cursed.trust < clean.trust);
    assert_eq!(cursed.as_, TookAs::Language);
    // A man who doesn't swear minds it aimed at him; an NFL locker room
    // doesn't.
    let quiet = Wants { hard: 0.2, swears: false, ..Wants::default() };
    let nfl = Wants { nfl: true, ..quiet };
    let at = line("firm", "profane", "work", "neutral", "tell");
    assert!(
        land(&at, &nfl, 0.5, None, &Setting::default()).trust > land(&at, &quiet, 0.5, None, &Setting::default()).trust
    );
    // A high school kid's parents hear the cursing.
    let kid = Wants { minor: true, ..competitor() };
    assert!(land(&soft(), &kid, 0.5, None, &Setting::default()).parents > 0.0);
}

#[test]
fn a_veteran_pushes_back_and_a_rookie_takes_it() {
    let vet = Wants { nfl: true, veteran: 1.0, hard: 0.3, swears: true, ..Wants::default() };
    let rookie = Wants { veteran: 0.0, ..vet };
    let v = land(&soft(), &vet, 0.3, None, &Setting::default());
    let r = land(&soft(), &rookie, 0.3, None, &Setting::default());
    assert_eq!(v.as_, TookAs::PushesBack);
    assert!(v.trust < r.trust);
}

#[test]
fn the_same_heat_every_day_wears_and_a_calm_coachs_explosion_lands_hardest() {
    let set = Setting::default();
    let used = land(&destroy(), &competitor(), 0.5, Some(destroy().heat), &set);
    let rare = land(&destroy(), &competitor(), 0.5, Some(0.1), &set);
    assert!(rare.mood > used.mood, "{used:?} {rare:?}");
}

#[test]
fn a_read_fades_over_about_a_season() {
    let mut a = Acc::default();
    let d = crate::calendar::Date(1000);
    a.add(destroy(), 2.0, d);
    let later = d.add(HALF_LIFE_DAYS as i32);
    assert!((a.weight(later) - 1.0).abs() < 0.01);
    assert!(a.weight(d.add(365)) < 0.25 * 2.0);
    // The average holds while the weight fades.
    assert!((a.mean().unwrap().heat - destroy().heat).abs() < 1e-5);
}

/// Your college program with one starter on it.
fn player() -> (World, u32) {
    let mut w = seat("college");
    let p = w.seat.and_then(|s| s.program).unwrap();
    w.add_class_aged(19);
    let year = w.year;
    let i = (0..w.people.len()).find(|&i| w.people[i].stage == Stage::HighSchool).unwrap();
    let q = &mut w.people[i];
    q.stage = Stage::College;
    q.college = Some(p);
    q.class_of = year - 2;
    q.college_seasons = 2;
    q.last_role = Some(PlayingTime::Starter);
    q.trust = 0.3;
    w.programs[p].roster.push(i);
    let id = w.people[i].player.id;
    (w, id)
}

#[test]
fn your_profile_reputation_is_the_start_and_your_lines_take_over() {
    let (mut w, id) = player();
    assert!(manner::your_read(&w).is_none(), "a blank start has no read");
    w.career.as_mut().unwrap().you.profile.manner = "professor".into();
    let (_, words) = manner::your_read(&w).unwrap();
    assert!(words.contains("Professor"), "{words}");
    // A freshman in week one knows your reputation, not you.
    let s = manner::seen_of(&w, Them::Player(id)).unwrap();
    assert!(s.own == 0.0 && s.weight > 1.0);
    // A season of Firebreather lines takes over.
    for k in 0..40 {
        w.today = w.today.add(5);
        let room = channels::one_on_one(&crate::world::folk::scene::Scene::new(
            crate::world::folk::scene::Kind::OneOnOne,
            w.today,
            "A talk",
        ));
        let _ = k;
        manner::said(&mut w, &room, destroy(), &[]);
    }
    let (_, words) = manner::your_read(&w).unwrap();
    assert!(words.starts_with("a Firebreather"), "{words}");
}

#[test]
fn a_typed_line_lands_its_manner_inside_the_caps_and_is_kept() {
    let (mut w, id) = player();
    let sid = talks::start_talk(&mut w, id, Subject::Role).unwrap();
    let s = talk::scene(&w, sid).cloned().unwrap();
    let room = channels::room_for(&w, &s);
    store::begin(&mut w, sid, room.clone());
    let reading = Reading { manner: Some(soft()), ..Reading::default() };
    let conv = Conversation {
        parts: Parts::today(),
        said: Said {
            scene: sid,
            words: "That was fucking soft. You embarrassed this whole room.".into(),
            reading,
            promises: Vec::new(),
        },
    };
    let t = conv.turn(&w, &room, &conv.said.reading);
    let took = t.manner.iter().find(|x| x.0 == Them::Player(id)).expect("he took the manner");
    assert!(took.1.trust < 0.0, "{took:?}");
    assert!(t.hits.iter().any(|h| h.hit.on == Them::Player(id) && h.trust < 0.0), "a hit inside the caps");
    let before = w.people[crate::world::folk::person(&w, id).unwrap()].trust;
    conv.apply(&mut w, &t);
    let after = w.people[crate::world::folk::person(&w, id).unwrap()].trust;
    assert!(after < before);
    let s = manner::seen_of(&w, Them::Player(id)).unwrap();
    assert!(s.own > 0.9, "he keeps the line: {s:?}");
    assert!(w.folk.manner.yours.w > 0.9);
    assert!(w.folk.manner.room.w > 0.0, "the grapevine carries it");
    // The stores save and load.
    let bytes = rmp_serde::to_vec_named(&w.folk.manner).unwrap();
    let back: manner::Book = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(back, w.folk.manner);
    assert!(manner::Book::default().is_empty(), "an empty book is never written");
}

#[test]
fn a_pattern_of_complaints_brings_a_review_and_a_firing_for_cause() {
    let (mut w, _) = player();
    let you = w.career.as_ref().unwrap().coach;
    let boss = crate::world::folk::boss::trust(&w, crate::world::folk::Team::seat(&w).unwrap());
    // Cursing alone never gets you fired: parents' calls start no review.
    for k in 0..8 {
        manner::complain(&mut w, Complaint::ParentCall, Them::Player(9000 + k));
    }
    let d = w.today;
    manner::day(&mut w, d);
    assert!(w.folk.manner.review.is_none());
    // Abuse complaints from five people do.
    for k in 0..5 {
        manner::complain(&mut w, Complaint::Abuse, Them::Player(100 + k));
    }
    w.coaches[you].wins = 0;
    w.coaches[you].losses = 0;
    manner::day(&mut w, d);
    assert!(w.folk.manner.review.is_some(), "an outside review opens");
    assert!(crate::world::folk::boss::trust(&w, crate::world::folk::Team::seat(&w).unwrap()) < boss);
    // More during the review: fired for cause, nothing owed.
    w.today = d.add(10);
    for k in 0..2 {
        manner::complain(&mut w, Complaint::Abuse, Them::Player(200 + k));
    }
    w.today = d.add(manner::REVIEW_DAYS);
    let later = w.today;
    manner::day(&mut w, later);
    assert!(w.coaches[you].job.is_none(), "fired");
    assert_eq!(w.coaches[you].history.last().map(|s| s.ended), Some("fired for cause"));
}

#[test]
fn ai_coaches_talk_from_their_traits_and_their_players_react_the_same_way() {
    let (w, _) = player();
    let you = w.career.as_ref().unwrap().coach;
    // An AI head coach somewhere: his manner from his temper and his way
    // with people.
    let c = w.programs.iter().filter_map(|p| p.head_coach).find(|&c| c != you).unwrap();
    let m = manner::ai(&w, c);
    for v in [m.heat, m.profane, m.aim, m.care, m.ask] {
        assert!((0.0..=1.0).contains(&v));
    }
    let mut w = w;
    w.coaches[c].personality.temper = 19.0;
    let h = manner::ai(&w, c);
    w.coaches[c].personality.temper = 3.0;
    let k = manner::ai(&w, c);
    assert!(h.heat > k.heat && h.aim > k.aim);
    // Your own players get yours line by line: no season read for you.
    let mine = w.programs[w.seat.and_then(|s| s.program).unwrap()].roster[0];
    assert_eq!(manner::ai_fit(&w, Some(you), mine), 0.0);
}

#[test]
fn a_blank_start_gives_recruits_nothing_and_a_new_coach_meets_a_fresh_trust() {
    let (mut w, id) = player();
    let you = w.career.as_ref().unwrap().coach;
    let i = w.people.iter().position(|p| p.player.id == id).unwrap();
    // Nobody knows how you talk before your first lines.
    assert!(manner::of_coach(&w, you).is_none());
    assert_eq!(manner::recruit_fit(&w, you, &w.people[i]), 0.0);
    // An AI coach new to him reads him at the camp read's fresh start,
    // not at what he had with his last coach.
    let c = w.programs.iter().filter_map(|p| p.head_coach).find(|&c| c != you).unwrap();
    w.people[i].trust_in = you as u32;
    w.people[i].trust = 0.0;
    let low = manner::ai_fit(&w, Some(c), i);
    w.people[i].trust = 1.0;
    let high = manner::ai_fit(&w, Some(c), i);
    assert_eq!(low, high, "the last coach's trust doesn't carry to a new one");
}
