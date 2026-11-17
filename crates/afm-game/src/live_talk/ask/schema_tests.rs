//! Every structured-output schema the game sends passes the API's rules
//! (`afm_ai::schema_problems`). A schema the API refuses fails the whole
//! call with a 400, as "Ask in your words" did on an enum declared
//! `["string", "null"]`. A new schema goes in this list.

use afm_ai::schema_problems;
use serde_json::Value;

use crate::lane_find::fields::Seat;
use crate::live_talk::{persona, say};

fn every_schema() -> Vec<(&'static str, Value)> {
    vec![
        ("ask analyst", super::analyst::schema(&[])),
        ("ask find, college", super::find::schema(Seat::College(0), &[])),
        ("ask find, nfl", super::find::schema(Seat::Nfl(0), &[])),
        ("ask leads", super::leads::schema()),
        ("ask opponent", super::opponent::schema()),
        ("ask source", super::source::reader_schema()),
        ("ask squad", super::squad::schema(&[])),
        ("say reader", say::reader_schema()),
        ("say one-on-one", say::one_on_one::schema()),
        ("say rooms", say::rooms::schema()),
        ("persona ear", persona::ear::schema()),
        ("persona voice", persona::voice::schema()),
        ("persona game-day words", persona::gameday::word_schema()),
    ]
}

#[test]
fn every_schema_passes_the_api_rules() {
    let bad: Vec<String> = every_schema()
        .into_iter()
        .flat_map(|(name, s)| schema_problems(&s).into_iter().map(move |p| format!("{name}: {p}")))
        .collect();
    assert!(bad.is_empty(), "schemas the API would refuse:\n{}", bad.join("\n"));
}

#[test]
fn the_search_levels_may_be_null() {
    // The field that failed: the college Find's level takes "hs" or null.
    let s = super::find::schema(Seat::College(0), &[]);
    let any = s["properties"]["level"]["anyOf"].as_array().unwrap();
    assert!(any[0]["enum"].as_array().unwrap().iter().any(|v| v == "hs"));
    assert_eq!(any[1]["type"], "null");
}
