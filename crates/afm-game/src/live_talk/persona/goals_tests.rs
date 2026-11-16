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
    assert!(!vc.may_name.iter().any(|n| n.text == me_name), "{:?}", vc.may_name);
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
    assert!(all.contains("What you want, most first"), "{all}");
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
    // "How's the family?" with nothing stored on his family: no news, not
    // "I don't know".
    f.push(EAR, Ok(ear(vec![act("question", "", "Life", "")], vec![], false)));
    f.push(VOICE, Ok(voice("They're good, Coach. Thanks for asking.", "")));
    let v = say(&mut g, id, "How is the family?");
    assert_eq!(v["state"], "done", "{v}");
    let card = last_voice(&f);
    assert!(card.contains("Nothing new at home to tell"), "{card}");
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
    assert_eq!(card.contains("What you ask the coach yourself (after you answer his line"), raises, "{card}");
    assert!(card.contains("He's your coach: you play for him"), "{card}");
}
