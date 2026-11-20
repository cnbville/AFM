//! Every person's voice plays toward his own goals (Cain 2026-11-15: "give
//! the AI targets"): a holdover coordinator knows his job is on the line
//! and who's who in the room, wants to keep it, and has his case and his
//! deal on the card, never a player's framing. And he answers what was
//! said, in his own situation, before he raises his own concern.

use afm_engine::world::folk::Team;
use afm_engine::world::folk::persona::Them;
use afm_engine::world::folk::persona::ear::Turn;
use afm_engine::world::folk::persona::talk::stakes::{self, Holdover};
use afm_engine::world::folk::persona::wants::WantKind;
use afm_engine::world::folk::scene::{self, Kind, Scene};
use afm_engine::world::folk::{facts, takeover};

use serde_json::json;

use super::super::{cards, ear::EAR, voice::VOICE};
use super::tests::{act, ear, game, say, talk, voice};

#[test]
fn a_holdover_coordinator_fights_for_his_job() {
    let (mut g, _, _) = game("goals-holdover", false);
    let w = g.world.as_mut().unwrap();
    let team = Team::seat(w).unwrap();
    let me = w.career.as_ref().unwrap().coach;
    let c = facts::assistants(w, team).into_iter().map(|x| x.0).find(|&c| c != me).expect("a staff coach");
    // The man you replaced: any other coach, gone from the team.
    let old = (0..w.coaches.len()).find(|&k| k != me && k != c && w.coaches[k].job.is_none()).unwrap_or(0);
    let today = w.today;
    let t = w.folk.takeover.get_or_insert_with(takeover::Takeover::default);
    t.team = Some(team);
    t.started = Some(today);
    t.old_coach = Some(old);
    t.kept.clear();
    t.let_go.clear();
    w.folk.staff.interviewed.retain(|x| x.0 != c);
    let mut s = Scene::new(Kind::Holdover, today, "Meeting the offensive coordinator");
    s.a = c as u64;
    s.stops = false;
    s.place = "Your office".into();
    let sid = scene::add(w, s);
    assert_eq!(stakes::holdover(w, c), Some(Holdover::OnTheLine));

    let s = w.folk.scenes.iter().find(|x| x.id == sid).unwrap().clone();
    let room = super::room_of(w, &s);
    let parts = super::parts_of(s.kind);
    let wants = parts.wanting.wants(w, Them::Coach(c));
    assert_eq!(wants[0].kind, WantKind::Security, "keeping his job comes first: {wants:?}");

    let (_, keys) = cards::ear(w, &s, &room, &parts, "Why should I keep you?");
    let vc = cards::voice(w, &s, &room, &parts, &Turn::default(), "Why should I keep you?", &keys);
    let all = vc.stakes.join(" | ");
    let you = scene::your_name(w);
    assert!(all.contains("This meeting is about your job"), "{all}");
    assert!(all.contains(&format!("{you}, the man talking with you, is your boss")), "{all}");
    assert!(all.contains("new head coach"), "{all}");
    assert!(all.contains("keeping your job"), "{all}");
    assert!(all.contains("What you can point to"), "{all}");
    assert!(!all.contains("playing time"), "no player framing: {all}");
    // He is a coach on the card, with his own makeup, not a blank.
    assert!(!vc.background.is_empty() && !vc.football.is_empty() && !vc.known_you.is_empty(), "{vc:?}");
    assert!(vc.coach.contains("head coach"), "{}", vc.coach);
    // You are never a third man in the room.
    let me_name = w.coaches[me].name.clone();
    assert!(!vc.may_name.iter().any(|n| n.text.split(',').next() == Some(me_name.as_str())), "{:?}", vc.may_name);
    let text = vc.text();
    assert!(text.contains("What this talk is to you"), "{text}");
    // The page tells you what's at stake, the way you'd know it.
    let about = stakes::situation(w, &s, Them::Coach(c)).join(" ");
    assert!(about.contains("his job is on the line"), "{about}");

    // Kept on: still coaching for his job, and it weighs less.
    w.folk.takeover.as_mut().unwrap().kept.push(c);
    assert_eq!(stakes::holdover(w, c), Some(Holdover::Kept));
    let kept = parts.wanting.wants(w, Them::Coach(c));
    let wt = |v: &[afm_engine::world::folk::persona::wants::Want]| {
        v.iter().find(|x| x.kind == WantKind::Security).map_or(0.0, |x| x.weight)
    };
    assert!(wt(&kept) < wt(&wants), "{kept:?}");
}

#[test]
fn your_own_player_keeps_a_players_goals() {
    let (g, _, him) = game("goals-player", false);
    let w = g.world.as_ref().unwrap();
    let mut s = Scene::new(Kind::OneOnOne, w.today, "A talk");
    s.a = him as u64;
    let parts = super::parts_of(s.kind);
    let lines = stakes::stakes(w, &s, Them::Player(him), &parts.wanting.wants(w, Them::Player(him)));
    let all = lines.join(" | ");
    assert!(all.contains("He's your coach: you play for him"), "{all}");
    assert!(all.contains("What you care about, most first"), "{all}");
    assert!(!all.contains("your job"), "{all}");
}

/// The voice's card text for the last line said.
fn last_voice(f: &afm_ai::Fake) -> String {
    f.asked().into_iter().rev().find(|r| r.label == VOICE).map(|r| r.user).unwrap_or_default()
}

#[test]
fn he_answers_what_was_said_in_his_own_situation() {
    let (mut g, f, him) = game("goals-coherent", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    // "How's the family?" with no trouble stored at home: his people, not
    // "I don't know" (personal talk, 2026-11-17).
    f.push(EAR, Ok(ear(vec![act("question", "", "Life", "")], vec![], false)));
    f.push(VOICE, Ok(voice("They're good, Coach. Thanks for asking.", "")));
    let v = say(&mut g, id, "How is the family?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(card.contains("Your life away from football"), "{card}");
    assert!(!card.contains("He doesn't know"), "{card}");
    // An offer gets his answer, not his own concern in its place.
    let mut offer = act("offer", "", "", "");
    offer["deal"] = json!("Favour");
    offer["words"] = json!("a few days off to see his family");
    f.push(EAR, Ok(ear(vec![offer], vec![], false)));
    f.push(VOICE, Ok(voice("I'd take that, Coach. Thank you.", "")));
    let v = say(&mut g, id, "How about I give you a few days off so you can connect with them?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(!card.contains("What you ask the coach yourself"), "{card}");
    // A plain line next: now he can bring up what's on his mind.
    f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Good, Coach. Where do I stand, though?", "")));
    let v = say(&mut g, id, "How are you feeling about things?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    // He raises his own concern once his heaviest unmet want is heavy enough.
    let w = g.world.as_ref().unwrap();
    let raises = super::parts_of(Kind::OneOnOne)
        .wanting
        .wants(w, Them::Player(him))
        .iter()
        .any(|x| !x.met && x.weight >= afm_engine::world::folk::persona::talk::ASK_WEIGHT);
    assert_eq!(card.contains("What you ask the coach yourself, after you answer the coach's line"), raises, "{card}");
    assert!(card.contains("He's your coach: you play for him"), "{card}");
}

#[test]
fn the_floor_you_give_him_and_a_line_with_one_bad_sentence() {
    let (mut g, f, him) = game("audit-floor", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    // A sentence with a number his card doesn't hold goes; the rest of his
    // line stays (the talks audit, 2026-11-16: the whole line went, and
    // the screen showed "Yeah.").
    f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Doing good, Coach. I ran for 412 yards against them.", "")));
    let v = say(&mut g, id, "How are you doing?");
    assert_eq!(v["state"], "done", "{v}");
    let w = g.world.as_ref().unwrap();
    let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
    let his = s.lines.iter().rev().find(|l| l.by > 1).unwrap();
    assert!(his.text.starts_with("Doing good, Coach.") && !his.text.contains("412"), "{:?}", his.text);
    // "Any questions?" gives him the floor: never "he doesn't know".
    f.push(EAR, Ok(ear(vec![act("question", "", "Nothing", "")], vec![], false)));
    f.push(VOICE, Ok(voice("Just one, Coach.", "")));
    let v = say(&mut g, id, "Any questions for me?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(!card.contains("He doesn't know"), "{card}");
    // Given the floor, he asks what's on his mind when anything weighs enough.
    let w = g.world.as_ref().unwrap();
    let raises = super::parts_of(Kind::OneOnOne)
        .wanting
        .wants(w, Them::Player(him))
        .iter()
        .any(|x| !x.met && x.weight >= afm_engine::world::folk::persona::talk::OPEN_FLOOR_ASK);
    assert_eq!(card.contains("What you ask the coach yourself, after you answer the coach's line"), raises, "{card}");
}

#[test]
fn he_raises_one_ask_at_a_time() {
    let (mut g, f, him) = game("audit-pace", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    // Plain lines, none on his wants: he may raise what's on his mind, but
    // never on the line right after an ask still on the table (the talks
    // audit, 2026-11-16: your trainer asked two new things back to back).
    // The voice presses every line, as the real one did after the card said
    // to let it go: "I'm still waiting on you to tell me where I stand". The
    // line right after his ask dodges it, the others take it up.
    let (mut raised, mut before, mut his, mut dodged) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for line in ["How are you doing?", "How's your week been?", "How are you holding up?", "How's it going?"] {
        let dodge = raised.last() == Some(&true);
        dodged.push(dodge);
        let reading =
            json!({ "acts": [act("question", "", "Mood", "")], "hits": [], "dodged": dodge, "not_talk": false });
        f.push(EAR, Ok(afm_ai::fake::body(&reading.to_string(), 1200, 120)));
        f.push(VOICE, Ok(voice("Doing good, Coach. I'm still waiting on you to tell me where I stand.", "")));
        let v = say(&mut g, id, line);
        if v["state"] != "done" {
            break;
        }
        let card = last_voice(&f);
        raised.push(card.contains("What you ask the coach yourself"));
        before.push(card.contains("Already asked earlier in this talk"));
        let w = g.world.as_ref().unwrap();
        let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
        his.push(s.lines.iter().rev().find(|l| l.by > 1).map(|l| l.text.clone()).unwrap_or_default());
    }
    assert!(!raised.is_empty());
    assert!(!raised[0], "never on his first line: {raised:?}");
    assert!(!raised.windows(2).any(|x| x[0] && x[1]), "{raised:?}");
    // Two lines on, an ask he raised is one he never asks again (the talks
    // audit: a coordinator asked where he stood on four lines of five).
    for k in 0..raised.len().saturating_sub(2) {
        if raised[k] {
            assert!(before[k + 2], "{raised:?} {before:?}");
        }
    }
    // Answered or let go, a press never reaches the screen; dodged on the
    // line after his ask, he may press it once.
    for k in 0..his.len() {
        if before[k] {
            assert_eq!(his[k], "Doing good, Coach.", "{k}: {his:?} {raised:?} {before:?} {dodged:?}");
        } else {
            assert!(his[k].contains("still waiting"), "{k}: {his:?}");
        }
        // An ask taken up is answered at once; one dodged is his to press.
        if k > 0 && raised[k - 1] {
            assert_eq!(before[k], !dodged[k], "{k}: {raised:?} {before:?} {dodged:?}");
        }
    }
}

#[test]
fn an_ask_the_coach_takes_up_is_answered() {
    let (mut g, f, him) = game("audit-answered", true);
    let f = f.unwrap();
    let id = talk(&mut g, him, "a check-in");
    // Lines until he raises an ask, then one that takes it up: his card
    // says it's answered, and pressing it again never reaches the screen
    // (the talks audit, 2026-11-16: "You're already my first pick at DT",
    // then "do I have a real role on this D-line or not?").
    let mut asked = false;
    for line in ["How are you doing?", "How's your week been?", "How are you holding up?", "How's it going?"] {
        f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
        f.push(VOICE, Ok(voice("Doing good, Coach.", "")));
        let v = say(&mut g, id, line);
        assert_eq!(v["state"], "done", "{v}");
        if last_voice(&f).contains("What you ask the coach yourself") {
            asked = true;
            break;
        }
    }
    assert!(asked, "given plain lines, he raises what's on his mind");
    f.push(EAR, Ok(ear(vec![act("question", "", "Mood", "")], vec![], false)));
    f.push(
        VOICE,
        Ok(voice("First pick, huh? But I need you to tell me plain, Coach, do I have a real role or not?", "")),
    );
    let v = say(&mut g, id, "You're already my first pick. I need more from you.");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(card.contains("He answered the question you asked him on your last line"), "{card}");
    let w = g.world.as_ref().unwrap();
    let s = w.folk.scenes.iter().find(|s| s.id == id).unwrap();
    let his = s.lines.iter().rev().find(|l| l.by > 1).unwrap();
    assert_eq!(his.text, "First pick, huh?", "{:?}", s.lines);
}
