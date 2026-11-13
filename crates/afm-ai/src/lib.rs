//! The live talks pipe (live talks lane 1): the one place the game's code
//! talks to the Claude API, and the one place its cost is counted.
//!
//! - `client`: the request and reply types, the `Model` trait, the real
//!   client on `ureq` and every way a call can fail.
//! - `fake`: a client that plays recorded replies, for tests and for the
//!   browser build (`AFM_AI_FAKE=1`). CI never calls the network.
//! - `spend`: the price table, and the month's spend book with the $50 cap.
//! - `Client` (here): a model plus the spend book. Every call checks the
//!   cap first and adds its cost after, whatever the reply was.
//!
//! No `Game` or `World` type lives in this crate, so the World Reader can
//! use the same client later.

pub mod client;
pub mod fake;
pub mod spend;

#[cfg(test)]
mod local_server_tests;

use std::sync::{Arc, Mutex};

pub use client::{BASE_URL, Failure, Key, MODEL, Model, Real, Reply, Request, TIMEOUT, Usage};
pub use fake::Fake;
pub use spend::{CAP_NANO, CAP_USD, MonthBook, Price, Spend, month_now};

/// A model and the spend book it is counted in.
#[derive(Clone)]
pub struct Client {
    model: Arc<dyn Model>,
    spend: Arc<Mutex<Spend>>,
}

impl Client {
    pub fn new(model: Arc<dyn Model>, spend: Arc<Mutex<Spend>>) -> Client {
        Client { model, spend }
    }

    pub fn spend(&self) -> &Arc<Mutex<Spend>> {
        &self.spend
    }

    /// True when the model behind it is the fake.
    pub fn is_fake(&self) -> bool {
        self.model.is_fake()
    }

    /// Makes one call. At the cap, or for a model with no price in the
    /// table, it fails before anything is sent. Whatever the model answers
    /// with usage in it is counted, a refusal or a cut-off reply included,
    /// because the API bills those. A reply that is no use (a refusal, cut
    /// off, empty, not the JSON the schema asked for) comes back as a failure.
    pub fn call(&self, req: &Request) -> Result<Reply, Failure> {
        let month = month_now();
        let price = Price::of(&req.model)
            .ok_or_else(|| Failure::Api { status: 0, message: format!("no price for {}", req.model) })?;
        if self.spend.lock().map_err(|_| poisoned())?.cap_reached(&month) {
            return Err(Failure::CapReached);
        }
        let mut reply = self.model.call(req)?;
        reply.nano = price.cost(&reply.usage);
        self.spend.lock().map_err(|_| poisoned())?.add(&month, &reply.usage, reply.nano);
        reply.classify(req)?;
        Ok(reply)
    }
}

fn poisoned() -> Failure {
    Failure::Api { status: 0, message: "the spend book stopped after an error".into() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn client(fake: Arc<Fake>, cap: u64) -> (Client, Arc<Mutex<Spend>>) {
        let spend = Arc::new(Mutex::new(Spend::memory().with_cap(cap)));
        (Client::new(fake, spend.clone()), spend)
    }

    #[test]
    fn every_call_is_counted_by_its_usage() {
        let fake = Arc::new(Fake::new());
        let (c, spend) = client(fake.clone(), CAP_NANO);
        let r = c.call(&Request::new("writer", "rules", "card")).unwrap();
        // The writer recording: 170 fresh, 680 from the cache, 34 out.
        assert_eq!(r.nano, 170 * 100 + 680 * 10 + 34 * 500);
        let month = month_now();
        let book = spend.lock().unwrap().book(&month);
        assert_eq!((book.calls, book.nano), (1, r.nano));
        c.call(&Request::new("test", "r", "u")).unwrap();
        assert_eq!(spend.lock().unwrap().book(&month).calls, 2);
    }

    #[test]
    fn at_the_cap_nothing_is_sent() {
        let fake = Arc::new(Fake::new());
        let (c, _) = client(fake.clone(), 1_000);
        // The first call is under the cap and takes it over; the second never leaves.
        c.call(&Request::new("writer", "r", "u")).unwrap();
        assert_eq!(c.call(&Request::new("writer", "r", "u")).unwrap_err(), Failure::CapReached);
        assert_eq!(fake.asked_count(), 1);
    }

    #[test]
    fn a_model_with_no_price_is_never_sent() {
        let fake = Arc::new(Fake::new());
        let (c, _) = client(fake.clone(), CAP_NANO);
        let err = c.call(&Request::new("writer", "r", "u").model("claude-haiku-4-5")).unwrap_err();
        assert!(matches!(err, Failure::Api { .. }));
        assert_eq!(fake.asked_count(), 0);
    }

    #[test]
    fn a_refusal_is_a_failure_but_is_still_paid_for() {
        let fake = Arc::new(Fake::new());
        fake.push(
            "reader",
            Ok(json!({
                "content": [], "stop_reason": "refusal",
                "usage": {"input_tokens": 1000, "output_tokens": 0}
            })),
        );
        let (c, spend) = client(fake, CAP_NANO);
        assert_eq!(c.call(&Request::new("reader", "r", "u")).unwrap_err(), Failure::Refusal);
        assert_eq!(spend.lock().unwrap().spent(&month_now()), 1000 * 100);
    }

    #[test]
    fn a_failure_with_no_reply_costs_nothing() {
        let fake = Arc::new(Fake::new());
        fake.push("writer", Err(Failure::Timeout));
        let (c, spend) = client(fake, CAP_NANO);
        assert_eq!(c.call(&Request::new("writer", "r", "u")).unwrap_err(), Failure::Timeout);
        assert_eq!(spend.lock().unwrap().spent(&month_now()), 0);
    }
}
