//! The ear's JSON reads as the contract says, and the fakes answer.

use super::*;
use crate::live_talk::filter;
use afm_engine::world::folk::persona::ear::Act;

const REPLY: &str = r#"{
  "acts": [
    {"act": "question", "topic": "Mood", "about": "p2", "words": "how's Tre doing"},
    {"act": "promise", "words": "we'll stunt you on a quarter of your rushes"},
    {"act": "end"}
  ],
  "hits": [{"item": "c2", "how": "reassured"}, {"on": "p2", "item": "c4", "how": "stung"}]
}"#;

#[test]
fn ear_reply_reads() {
    let r: EarReply = serde_json::from_str(REPLY).unwrap();
    assert_eq!(r.acts.len(), 3);
    assert_eq!(r.acts[0].act, ActKind::Question);
    assert_eq!(r.acts[0].topic, Some(Topic::Mood));
    assert_eq!(r.acts[1].act, ActKind::Promise);
    assert_eq!(r.hits[0].on, None);
    assert_eq!(r.hits[1].how, How::Stung);
    assert!(!r.dodged && !r.not_talk);
    let back: EarReply = serde_json::from_value(serde_json::to_value(&r).unwrap()).unwrap();
    assert_eq!(back, r);
}

#[test]
fn fakes_answer() {
    let reply: EarReply = serde_json::from_str(REPLY).unwrap();
    assert_eq!(FakeEar(Some(reply.clone())).read(&EarCard::default()), Ok(reply.clone()));
    assert_eq!(FakeEar(None).read(&EarCard::default()), Err(Failure::NoKey));
    let reading = Reading { acts: vec![Act::End], ..Default::default() };
    assert_eq!(FakeResolve(reading.clone()).resolve(&reply, &EarKeys::default()), reading);
    let said = FakeVoice(Some("Coach, I hear you.".into())).say(&VoiceCard::default()).unwrap();
    // The voice's words still pass the reply filter before the screen.
    assert!(filter::check(&said, "").is_ok());
    assert_eq!(FakeVoice(None).say(&VoiceCard::default()), Err(Failure::NoKey));
}

#[test]
fn an_offer_reads_its_gives_and_terms() {
    let json = r#"{"acts": [{"act": "offer", "deal": "Nil", "words": "the 3rd-down job and a scholarship for your brother",
        "gives": [{"give": "role"}, {"give": "scholarship_for", "who": "p2"}, {"give": "money", "number": 40000},
                  {"give": "jersey"}, {"give": "spaceship"}],
        "terms": [{"measure": "m1", "per": "share", "number": 0.25, "most": false, "spot": "any", "down": 3,
                   "package": "none", "look": "none"},
                  {"measure": "chips", "per": "game", "number": 2}]}]}"#;
    let r: EarReply = serde_json::from_str(json).unwrap();
    let keys = EarKeys {
        people: vec![("p1".into(), Them::Player(7)), ("p2".into(), Them::Player(9))],
        measures: vec![("m1".into(), "snaps".into())],
        ..Default::default()
    };
    let a = &r.acts[0];
    let gives: Vec<Give> = a.gives.iter().filter_map(|g| g.give(&keys)).collect();
    // A jersey with no number and a key the schema doesn't hold are dropped.
    assert_eq!(gives, vec![Give::Role(1), Give::Scholarship(Some(9)), Give::Money(40000.0)]);
    let terms: Vec<_> = a.terms.iter().filter_map(|t| t.usage(Position::RB, &keys)).collect();
    // Chips are the other team's call: never a term.
    assert_eq!(terms.len(), 1);
    assert_eq!(terms[0].down, 3);
    assert!(terms[0].share && (terms[0].n - 0.25).abs() < 1e-6);
}

#[test]
fn the_draft_sides_requests_read() {
    use afm_engine::world::folk::persona::terms::Request;
    let json = r#"{"acts": [
        {"act": "request", "what": "scout", "about": "p2", "topic": "Character"},
        {"act": "request", "what": "scout", "about": "p2"},
        {"act": "request", "what": "find_out", "about": "p2"},
        {"act": "request", "what": "whiteboard"},
        {"act": "request", "what": "plant", "about": "p2"},
        {"act": "request", "what": "plant"}]}"#;
    let r: EarReply = serde_json::from_str(json).unwrap();
    let keys =
        EarKeys { people: vec![("p1".into(), Them::Player(7)), ("p2".into(), Them::Player(9))], ..Default::default() };
    let tags = ear::Tags { today: afm_engine::calendar::Date::ymd(2030, 3, 1), pos: None };
    let got: Vec<Request> = tags
        .resolve(&r, &keys)
        .acts
        .into_iter()
        .filter_map(|a| match a {
            Act::Request(x) => Some(x),
            _ => None,
        })
        .collect();
    // A plant with nobody named drops.
    assert_eq!(
        got,
        vec![
            Request::FindOut(9, Topic::Character),
            Request::Scout(9),
            Request::FindOut(9, Topic::ScoutFile),
            Request::Whiteboard,
            Request::Plant(9),
        ]
    );
}

#[test]
fn a_game_plan_ask_names_its_setting() {
    use afm_engine::world::folk::persona::terms::Request;
    // The talks audit, 2026-11-16: extra treatment, asked of the doctor,
    // came back as a game plan ask he turned down.
    let json = r#"{"acts": [
        {"act": "request", "what": "plan", "value": "", "words": "get the banged-up guys extra treatment this week"},
        {"act": "request", "what": "plan", "value": "more pressure, a lot"},
        {"act": "request", "what": "plan", "value": "", "words": "Run the ball more"}]}"#;
    let r: EarReply = serde_json::from_str(json).unwrap();
    let keys = EarKeys { people: vec![("p1".into(), Them::Player(7))], ..Default::default() };
    let tags = ear::Tags { today: afm_engine::calendar::Date::ymd(2030, 3, 1), pos: None };
    let got: Vec<Request> = tags
        .resolve(&r, &keys)
        .acts
        .into_iter()
        .filter_map(|a| match a {
            Act::Request(x) => Some(x),
            _ => None,
        })
        .collect();
    assert_eq!(
        got,
        vec![
            Request::Other("get the banged-up guys extra treatment this week".into()),
            Request::Plan("more pressure, a lot".into()),
            Request::Plan("Run the ball more".into()),
        ]
    );
}

#[test]
fn an_ask_for_his_read_is_a_request_and_a_decision_on_another_man_drops() {
    use afm_engine::world::folk::persona::terms::{Decision, Request};
    // "Take a look at Strand, I want to decide if we should cut him", to
    // his position coach: his read on Strand, never a cut of the coach.
    let json = r#"{"acts": [
        {"act": "request", "what": "evaluate", "about": "p2"},
        {"act": "decision", "what": "cut", "about": "p2"},
        {"act": "decision", "what": "coach_let_go", "about": "p1"},
        {"act": "decision", "what": "coach_let_go"},
        {"act": "request", "what": "evaluate"}]}"#;
    let r: EarReply = serde_json::from_str(json).unwrap();
    let keys =
        EarKeys { people: vec![("p1".into(), Them::Coach(3)), ("p2".into(), Them::Player(9))], ..Default::default() };
    let tags = ear::Tags { today: afm_engine::calendar::Date::ymd(2030, 3, 1), pos: None };
    let got = tags.resolve(&r, &keys).acts;
    // A decision tagged on someone else is news about him, not a decision
    // on the man across; an evaluation with nobody named drops.
    assert_eq!(
        got,
        vec![
            Act::Request(Request::Evaluate(9)),
            Act::Decision(Decision::CoachLetGo),
            Act::Decision(Decision::CoachLetGo),
        ]
    );
    assert!(ear::EAR_RULES.contains("Still deciding is never a decision"));
}

#[test]
fn a_thank_you_ends_a_talk_only_with_a_goodbye() {
    let end = Reading { acts: vec![Act::End], ..Default::default() };
    let kept = |line: &str| ear::keep_open(end.clone(), line).acts;
    // The talks audit, 2026-11-16: thanks mid-talk was heard as the coach
    // closing.
    assert!(kept("I wanted to say thanks for everything you do for this program.").is_empty());
    assert!(kept("Appreciate you coming by. How's the family?").is_empty());
    for bye in [
        "Thanks, that's all for today.",
        "I appreciate your time tonight.",
        "Thanks for calling, let me know if you need anything else.",
        "Thanks, I\u{2019}ll let you get back to it.",
        "Thanks, keep me posted.",
    ] {
        assert_eq!(kept(bye), vec![Act::End], "{bye}");
    }
    // A goodbye with no thanks in it is the ear's to read.
    assert_eq!(kept("Good talk. My door's always open."), vec![Act::End]);
    assert_eq!(kept("Alright, let's go to work."), vec![Act::End]);
}

#[test]
fn his_line_never_reads_his_card_out() {
    let card = "What the game decided you say this turn (say all of it, add nothing):\n\
        - He writes it down: the goal is now to beat the rival. The coach is judged on it.\n\
        - He'll believe it when he sees it.\n\
        What you ask the coach yourself:\n\
        - he asks about his money\n";
    // The talks audit, 2026-11-16: your boss read his card out ("He writes
    // it down: the goal is now..."), a parent said "He asks about his money
    // first, too."
    let said = "Good, Test, write it down. He writes it down: the goal is now to beat the rival, and you're judged on it. I'll believe it when I see it.";
    assert_eq!(
        voice::unparrot(said, card).as_deref(),
        Some("Good, Test, write it down. I'll believe it when I see it.")
    );
    assert_eq!(voice::unparrot("He asks about his money first, too.", card), None);
    assert_eq!(voice::unparrot("Fine, Coach.", card).as_deref(), Some("Fine, Coach."));
    // A short line counts whole: "He doesn't know, Coach." from a senior
    // asked what he wants done before the draft.
    let short = "What the game decided you say this turn:\n- He doesn't know.\n";
    assert_eq!(
        voice::unparrot("He doesn't know, Coach. What would you tell the scouts?", short).as_deref(),
        Some("What would you tell the scouts?")
    );
    assert_eq!(voice::unparrot("He knows the plan.", short).as_deref(), Some("He knows the plan."));
    // "He'll" and "He'd" lines too, said as himself where they can be:
    // "Yes, Coach, I'll do it. He'll do it." and "He'd rather not get into
    // it, but yeah, I'm trying to hear what y'all can offer me" (the talks
    // audit, 2026-11-16).
    let will = "What the game decided you say this turn:\n- He'll do it.\n- He\u{2019}d rather not get into it.\n- He'll need your offer before he can commit.\n";
    assert_eq!(
        voice::unparrot("Yes, Coach, I'll do it. He'll do it.", will).as_deref(),
        Some("Yes, Coach, I'll do it.")
    );
    assert_eq!(
        voice::unparrot("He'd rather not get into it, but yeah, I'm trying to hear what y'all can offer me.", will)
            .as_deref(),
        Some("I'd rather not get into it, but yeah, I'm trying to hear what y'all can offer me.")
    );
    assert_eq!(
        voice::unparrot("He'll need your offer before he can commit. Thanks, Coach.", will).as_deref(),
        Some("Thanks, Coach.")
    );
    // His parents answering talk about their son as "he", as they should.
    let theirs = format!("- This line is answered by Andrew's parents (his parents), not by you.\n{will}");
    assert_eq!(voice::unparrot("He'll do it, Coach.", &theirs).as_deref(), Some("He'll do it, Coach."));
}

#[test]
fn a_press_he_let_go_never_reaches_the_screen() {
    // The talks audit, 2026-11-16: told on his card the ask was let go, a
    // coordinator still pressed it on every line after.
    assert_eq!(
        voice::let_go(
            "Family's doing fine, Coach, thanks for asking. Kids are settled. But you didn't come here to hear about my family, and I'm still waiting on you to tell me where I stand."
        )
        .as_deref(),
        Some("Family's doing fine, Coach, thanks for asking. Kids are settled.")
    );
    assert_eq!(
        voice::let_go(
            "Appreciate that, Coach, but a good week doesn't answer my question. I'm still asking how safe my place is here."
        )
        .as_deref(),
        Some("Appreciate that, Coach.")
    );
    assert_eq!(voice::let_go("I\u{2019}m still asking where I stand."), None);
    // Waiting on something else isn't pressing the coach.
    let mri = "We're still waiting on the MRI for the ankle. I'll know Friday.";
    assert_eq!(voice::let_go(mri).as_deref(), Some(mri));
}

#[test]
fn a_last_line_asks_nothing_new() {
    // The last replay, 2026-11-17: a college coach's last line on the call
    // asked a new question, and the call ended with nobody to answer it.
    assert_eq!(
        voice::no_new_question(
            "Not sure I should speak for my staff on a kid we haven't evaluated yet, Coach. Is there anything else I should know about Thomas before we hang up?"
        ),
        "Not sure I should speak for my staff on a kid we haven't evaluated yet, Coach."
    );
    assert_eq!(voice::no_new_question("Can I call you next week? Talk soon, Coach."), "Talk soon, Coach.");
    // A question that is all he said stays.
    assert_eq!(voice::no_new_question("You'll call me back?"), "You'll call me back?");
}

#[test]
fn a_request_reads_as_what_hes_asked_to_do() {
    // The talks audit, 2026-11-16: "He agreed to I want you to spend more
    // time with the young corners this week.." reached the scene.
    let said = "I want you to spend more time with the young corners this week.";
    assert_eq!(ear::ask_phrase(said), "spend more time with the young corners this week");
    assert_eq!(ear::ask_phrase("Can you please talk to Jace?"), "talk to Jace");
    assert_eq!(ear::ask_phrase("spend more time with the corners"), "spend more time with the corners");
    assert_eq!(ear::ask_phrase("Run the scout team"), "run the scout team");
    assert_eq!(ear::ask_phrase("I\u{2019}d like you to sit with Jace\u{2019}s group."), "sit with Jace's group");
    // What the coach wants more of is what he's asked to give (Cain's talk
    // with his DT, 2026-11-16), and to a player it's never a game plan.
    assert_eq!(ear::ask_phrase("I need more pressures from you"), "give you more pressures");
    use afm_engine::world::folk::persona::terms::Request;
    let r = afm_engine::world::folk::persona::ear::Reading {
        acts: vec![Act::Request(Request::Plan("I need more pressures from you".into()))],
        ..Default::default()
    };
    assert_eq!(ear::own_work(r).acts, vec![Act::Request(Request::Other("give you more pressures".into()))]);
}

#[test]
fn the_scene_reads_from_his_side_on_his_card() {
    // "Lowell Biggerstaff comes by your office": he said "come on in"
    // though he was the one who came (the talks audit, 2026-11-16).
    assert_eq!(
        cards::his_side("Lowell Biggerstaff comes by your office."),
        "Lowell Biggerstaff comes by the coach's office."
    );
    assert_eq!(cards::his_side("Your boss wants a word."), "The coach's boss wants a word.");
    assert_eq!(cards::his_side("In comes Tre. He sits down."), "In comes Tre. He sits down.");
}
