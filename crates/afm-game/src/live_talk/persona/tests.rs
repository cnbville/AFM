//! The ear's JSON reads as the contract says, and the fakes answer.

use super::*;
use crate::live_talk::filter;
use afm_engine::world::folk::persona::ear::Act;

const REPLY: &str = r#"{
  "acts": [
    {"act": "question", "topic": "Mood", "about": "p2", "words": "how's Tre doing"},
    {"act": "promise", "terms": [{"measure": "m1", "number": 0.25, "bound": "AtLeast",
      "scope": "game", "from": "now", "as_long_as": "", "words": "we'll stunt you on a quarter of your rushes"}]},
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
    assert_eq!(r.acts[1].terms[0].bound, Some(Bound::AtLeast));
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
