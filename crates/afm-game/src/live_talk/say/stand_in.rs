//! The browser build's stand-in (`AFM_AI_FAKE=1`): with no key, a typed
//! line still lands where its words point, so every screen can be tried.
//! The reader is a plain word match against the options on the card; the
//! writer says the game's own line back, so the bank's line stays. Never
//! used with a key.

use afm_ai::fake::body;
use serde_json::{Value, json};

use crate::live_talk::card::Card;

/// Words that tell nothing about which option a line means.
const SKIP: [&str; 24] = [
    "about", "would", "what", "your", "with", "that", "this", "have", "will", "just", "they", "them", "their", "there",
    "here", "when", "where", "want", "know", "think", "really", "coach", "tell", "like",
];

fn stems(t: &str) -> Vec<String> {
    t.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4 && !SKIP.contains(w))
        .map(|w| w.chars().take(5).collect())
        .collect()
}

/// The option whose words the line shares most, as a reader's answer.
pub fn reading(card: &Card, words: &str) -> Value {
    let said = stems(words);
    let best = card
        .options
        .iter()
        .map(|o| (stems(&o.text).iter().filter(|w| said.contains(w)).count(), o))
        .filter(|(n, _)| *n > 0)
        .max_by_key(|(n, _)| *n);
    let v = json!({
        "pick": best.map_or("none", |(_, o)| o.tag.as_str()),
        "also": "none",
        "tone": "calm",
        "promise": null,
        "claims": [],
        "named": [],
        "fit": if best.is_some() { "clear" } else { "not_a_line" },
    });
    body(&v.to_string(), 600, 60)
}

/// The writer's stand-in: the game's own line back.
pub fn echo(line: &str) -> Value {
    body(&afm_engine::world::folk::plain(line), 500, 40)
}

pub fn reading_value(card: &Card, words: &str) -> Value {
    let b = reading(card, words);
    b.pointer("/content/0/text").and_then(Value::as_str).and_then(|t| serde_json::from_str(t).ok()).unwrap_or(Value::Null)
}
