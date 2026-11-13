//! The one real test of the pipe (live talks lane 1): three calls to the
//! real API on `claude-haiku-5-5`, about a cent in all. Ignored, so CI never
//! runs it. Run it alone, with a key in `AFM_AI_KEY`:
//!
//! `AFM_AI_KEY="$AFM_LINEBANK_KEY" cargo test --release -p afm-ai real_ -- --ignored --nocapture`
//!
//! With `AFM_AI_KEY` empty it prints "skipped" and passes. On a thread machine
//! the call goes through the proxy the environment names (`AFM_AI_PROXY=1`);
//! on a Mac there is none. It prints the usage of each reply and never the
//! key. `AFM_AI_KEEP=<folder>` keeps each reply's JSON body there, as the
//! fake's recordings.

use afm_ai::{Fake, Key, Model, Price, Real, Request};
use serde_json::Value;

/// The reader's rules and schema from the proposal's appendix, and its one
/// sample line.
const RULES: &str = include_str!("reader_rules.txt");
const SCHEMA: &str = include_str!("reader_schema.json");
const SAMPLE: &str = include_str!("reader_sample.txt");

#[test]
#[ignore = "calls the real API; run alone with AFM_AI_KEY set"]
fn real_three_calls() {
    let raw = std::env::var("AFM_AI_KEY").unwrap_or_default();
    if raw.trim().is_empty() {
        println!("skipped: AFM_AI_KEY is empty");
        return;
    }
    let key = Key::parse(&raw).expect("AFM_AI_KEY holds an Anthropic key");
    let mut real = Real::new(key).with_timeout(std::time::Duration::from_secs(30));
    if std::env::var("AFM_AI_PROXY").is_ok_and(|v| v == "1") {
        real = real.with_proxy_from_env();
    }
    let schema: Value = serde_json::from_str(SCHEMA).expect("the schema is JSON");
    let price = Price::of("claude-haiku-5-5").unwrap();
    let calls = [
        ("test", Request::new("test", "Reply with the single word ok.", "ping").no_cache().max_tokens(16)),
        ("reader_1", Request::new("reader", RULES, SAMPLE).schema(schema.clone()).max_tokens(400)),
        ("reader_2", Request::new("reader", RULES, SAMPLE).schema(schema).max_tokens(400)),
    ];
    let mut total = 0u64;
    for (name, req) in calls {
        let mut reply = real.call(&req).unwrap_or_else(|e| panic!("{name} failed: {e:?}"));
        reply.nano = price.cost(&reply.usage);
        total += reply.nano;
        let u = reply.usage;
        println!(
            "{name}: stop={} in={} out={} cache_read={} cache_write_5m={} cache_write_1h={} cost=${:.6} {} ms",
            reply.stop,
            u.input,
            u.output,
            u.cache_read,
            u.cache_write_5m,
            u.cache_write_1h,
            reply.nano as f64 / 1e9,
            reply.millis
        );
        reply.classify(&req).unwrap_or_else(|e| panic!("{name} was no use: {e:?}"));
        if let Some(j) = &reply.json {
            println!("{name} reading: {j}");
        } else {
            println!("{name} text: {}", reply.text);
        }
        if let Ok(dir) = std::env::var("AFM_AI_KEEP") {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(format!("{dir}/{name}.json"), reply.body.to_string());
        }
    }
    println!("total ${:.6}", total as f64 / 1e9);
    assert!(total < 10_000_000, "under a cent");
    // The built-in recordings are the same shape as what came back.
    assert!(Fake::new().call(&Request::new("test", "r", "u")).is_ok());
}
