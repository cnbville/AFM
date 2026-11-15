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
