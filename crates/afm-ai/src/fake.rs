//! A client that plays recorded replies. CI never calls the network, so
//! every test of a live talk runs on this, and so does the browser build
//! when it is started with `AFM_AI_FAKE=1`.
//!
//! The first recordings (`recordings/`) are written by hand from the
//! proposal's sample prompts (the reader's one line and the writer's card).
//! When the real test has run, its replies are kept beside them. A test can
//! queue its own replies and failures per label, and read back every request
//! the fake was asked, which is how a test proves what a card held.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{Value, json};

use crate::client::{Failure, Model, Reply, Request};

const TEST: &str = include_str!("../recordings/test.json");
const READER: &str = include_str!("../recordings/reader.json");
const WRITER: &str = include_str!("../recordings/writer.json");

#[derive(Default)]
struct Inner {
    queue: HashMap<String, VecDeque<Result<Value, Failure>>>,
    asked: Vec<Request>,
}

/// The fake. Cheap to make; share it with `Arc` to read what it was asked.
pub struct Fake {
    inner: Mutex<Inner>,
    delay: Duration,
}

impl Default for Fake {
    fn default() -> Fake {
        Fake::new()
    }
}

impl Fake {
    /// No delay: for tests.
    pub fn new() -> Fake {
        Fake { inner: Mutex::new(Inner::default()), delay: Duration::ZERO }
    }

    /// A little latency, like the real thing, so a screen can show its
    /// waiting state: for the browser build.
    pub fn for_browser() -> Fake {
        Fake { inner: Mutex::new(Inner::default()), delay: Duration::from_millis(700) }
    }

    /// Queues one reply (an API body) or one failure for a label. Queued
    /// replies for a label are played in order; when none is left the
    /// built-in recording for that label plays.
    pub fn push(&self, label: &str, reply: Result<Value, Failure>) {
        if let Ok(mut g) = self.inner.lock() {
            g.queue.entry(label.to_string()).or_default().push_back(reply);
        }
    }

    /// Queues a plain text reply with a small usage.
    pub fn push_text(&self, label: &str, text: &str) {
        self.push(label, Ok(body(text, 100, 20)));
    }

    /// Every request asked so far, in order.
    pub fn asked(&self) -> Vec<Request> {
        self.inner.lock().map(|g| g.asked.clone()).unwrap_or_default()
    }

    pub fn asked_count(&self) -> usize {
        self.inner.lock().map_or(0, |g| g.asked.len())
    }

    /// The built-in recording for a label.
    pub fn recording(label: &str) -> Value {
        let raw = match label {
            "test" => TEST,
            "reader" => READER,
            "writer" => WRITER,
            _ => return body("Fine.", 100, 5),
        };
        serde_json::from_str(raw).unwrap_or_else(|_| body("Fine.", 100, 5))
    }
}

/// A reply body in the API's shape.
pub fn body(text: &str, input: u64, output: u64) -> Value {
    json!({
        "id": "msg_fake",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-5-5",
        "content": [{ "type": "text", "text": text }],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": input,
            "output_tokens": output,
            "cache_read_input_tokens": 0,
            "cache_creation_input_tokens": 0
        }
    })
}

impl Model for Fake {
    fn call(&self, req: &Request) -> Result<Reply, Failure> {
        let queued = {
            let mut g = self.inner.lock().map_err(|_| Failure::NoNetwork)?;
            g.asked.push(req.clone());
            g.queue.get_mut(&req.label).and_then(VecDeque::pop_front)
        };
        if !self.delay.is_zero() {
            std::thread::sleep(self.delay);
        }
        let b = match queued {
            Some(Ok(b)) => b,
            Some(Err(f)) => return Err(f),
            None => Fake::recording(&req.label),
        };
        Reply::from_body(b, self.delay.as_millis() as u64)
    }

    fn is_fake(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_recordings_are_whole_replies() {
        let f = Fake::new();
        let t = f.call(&Request::new("test", "r", "u")).unwrap();
        assert_eq!(t.text, "ok");
        assert!(t.usage.output > 0);
        // The reader's recording is the JSON the schema in the proposal asks for.
        let req = Request::new("reader", "r", "u").schema(json!({"type": "object"}));
        let mut r = f.call(&req).unwrap();
        r.classify(&req).unwrap();
        let j = r.json.unwrap();
        for k in ["move", "tone", "subject", "promise", "claims", "named", "fit"] {
            assert!(j.get(k).is_some(), "the reader's reading has {k}");
        }
        assert_eq!(j["move"], "straight");
        assert!(f.call(&Request::new("writer", "r", "u")).unwrap().text.contains("thirty games"));
        // An unknown label still answers.
        assert_eq!(f.call(&Request::new("anything", "r", "u")).unwrap().text, "Fine.");
        assert_eq!(f.asked_count(), 4);
    }

    #[test]
    fn queued_replies_play_in_order_then_the_recording() {
        let f = Fake::new();
        f.push_text("writer", "first");
        f.push("writer", Err(Failure::Timeout));
        let ask = Request::new("writer", "r", "u");
        assert_eq!(f.call(&ask).unwrap().text, "first");
        assert_eq!(f.call(&ask).unwrap_err(), Failure::Timeout);
        assert!(f.call(&ask).unwrap().text.contains("thirty games"));
        assert_eq!(f.asked()[0].label, "writer");
    }
}
