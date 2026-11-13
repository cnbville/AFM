//! The request and reply types, the `Model` trait and the real client.
//!
//! The real client sends one POST to `/v1/messages` on `ureq`, with the
//! model's fixed rules cached, thinking off and, when the caller gives a
//! schema, structured output. The whole call, connecting and reading the
//! answer included, is cut off after `TIMEOUT`. The key is read only here,
//! and nothing here prints it.

use std::fmt;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// Where the API is. Written out on purpose: thread machines set
/// `ANTHROPIC_BASE_URL`, and the game must never read it.
pub const BASE_URL: &str = "https://api.anthropic.com";

/// The only model the game's live talks use (Cain 2026-11-07: Haiku 5.5).
pub const MODEL: &str = "claude-haiku-5-5";

/// The API version header. The long-standing one: structured output and the
/// thinking switch need no newer one (checked by the real test).
pub const VERSION: &str = "2023-06-01";

/// The whole call, connecting and reading the answer included.
pub const TIMEOUT: Duration = Duration::from_secs(6);

/// An Anthropic API key. It can be made from text and nothing else can be
/// made of it: no `Display`, no `Serialize`, and `Debug` shows only the last
/// four characters.
#[derive(Clone)]
pub struct Key(String);

impl Key {
    /// Checks the shape (`sk-ant-`, letters, digits, `-` and `_` only, no
    /// spaces) and keeps it. The text is trimmed first, since a pasted key
    /// often carries a newline.
    pub fn parse(raw: &str) -> Result<Key, String> {
        let t = raw.trim();
        let ok = t.starts_with("sk-ant-")
            && (20..=300).contains(&t.len())
            && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if ok { Ok(Key(t.to_string())) } else { Err("That doesn't look like an Anthropic key. It starts with sk-ant-.".into()) }
    }

    /// The last four characters, for "Key ending ...abcd".
    pub fn tail(&self) -> String {
        let n = self.0.chars().count();
        self.0.chars().skip(n.saturating_sub(4)).collect()
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    /// Text with this key taken out of it, for error messages.
    pub fn scrub(&self, text: &str) -> String {
        text.replace(self.expose(), "...")
    }
}

impl fmt::Debug for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Key(...{})", self.tail())
    }
}

/// What to ask. `label` says what the call is for ("test", "reader",
/// "writer"): the fake picks its recording by it and logs show it. It is
/// never sent.
#[derive(Clone, Debug)]
pub struct Request {
    pub label: String,
    pub model: String,
    /// The fixed rules (the system text). Cached when long enough.
    pub rules: String,
    pub cache_rules: bool,
    /// The variable part: the card and the line.
    pub user: String,
    /// A JSON schema for structured output, or none for free text.
    pub schema: Option<Value>,
    pub max_tokens: u32,
}

impl Request {
    pub fn new(label: &str, rules: &str, user: &str) -> Request {
        Request {
            label: label.to_string(),
            model: MODEL.to_string(),
            rules: rules.to_string(),
            cache_rules: true,
            user: user.to_string(),
            schema: None,
            max_tokens: 512,
        }
    }

    pub fn schema(mut self, schema: Value) -> Request {
        self.schema = Some(schema);
        self
    }

    pub fn max_tokens(mut self, n: u32) -> Request {
        self.max_tokens = n;
        self
    }

    pub fn model(mut self, model: &str) -> Request {
        self.model = model.to_string();
        self
    }

    /// For a call whose rules are too short to cache.
    pub fn no_cache(mut self) -> Request {
        self.cache_rules = false;
        self
    }

    /// The request body, as the API takes it.
    pub fn body(&self) -> Value {
        let mut rules = json!({ "type": "text", "text": self.rules });
        if self.cache_rules {
            rules["cache_control"] = json!({ "type": "ephemeral" });
        }
        let mut b = json!({
            "model": self.model,
            "max_tokens": self.max_tokens,
            "thinking": { "type": "disabled" },
            "system": [rules],
            "messages": [{ "role": "user", "content": self.user }],
        });
        if let Some(s) = &self.schema {
            b["output_config"] = json!({ "format": { "type": "json_schema", "schema": s } });
        }
        b
    }
}

/// What a reply's `usage` says, in tokens.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write_5m: u64,
    pub cache_write_1h: u64,
}

impl Usage {
    pub fn from_body(u: &Value) -> Usage {
        let n = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        // The write is split by lifetime when the API says so; otherwise it
        // is all the 5-minute kind, which is all the game asks for.
        let (w5, w1) = match u.get("cache_creation").filter(|c| c.is_object()) {
            Some(c) => {
                let g = |k: &str| c.get(k).and_then(Value::as_u64).unwrap_or(0);
                (g("ephemeral_5m_input_tokens"), g("ephemeral_1h_input_tokens"))
            }
            None => (n("cache_creation_input_tokens"), 0),
        };
        Usage {
            input: n("input_tokens"),
            output: n("output_tokens"),
            cache_read: n("cache_read_input_tokens"),
            cache_write_5m: w5,
            cache_write_1h: w1,
        }
    }

    pub fn cache_write(&self) -> u64 {
        self.cache_write_5m + self.cache_write_1h
    }

    /// The whole prompt: what was read fresh, from the cache and written.
    pub fn prompt(&self) -> u64 {
        self.input + self.cache_read + self.cache_write()
    }
}

/// A reply the API gave (HTTP 200). A refusal or a cut-off reply is still a
/// reply here, because it is billed; `Reply::classify` turns the useless
/// ones into failures after they are counted.
#[derive(Clone, Debug)]
pub struct Reply {
    /// All the text blocks, joined.
    pub text: String,
    /// The text read as JSON, when the request had a schema.
    pub json: Option<Value>,
    pub stop: String,
    pub usage: Usage,
    /// What it cost, in billionths of a dollar. Filled in by `Client`.
    pub nano: u64,
    pub millis: u64,
    pub model: String,
    /// The API's body as it came, for recordings. It holds no key.
    pub body: Value,
}

impl Reply {
    pub fn from_body(body: Value, millis: u64) -> Result<Reply, Failure> {
        if body.get("type").and_then(Value::as_str) == Some("error") {
            let m = body.pointer("/error/message").and_then(Value::as_str).unwrap_or("error");
            return Err(Failure::Api { status: 0, message: m.to_string() });
        }
        let text: String = body
            .get("content")
            .and_then(Value::as_array)
            .map(|blocks| {
                blocks
                    .iter()
                    .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                    .filter_map(|b| b.get("text").and_then(Value::as_str))
                    .collect::<Vec<_>>()
                    .join("")
            })
            .unwrap_or_default();
        Ok(Reply {
            text,
            json: None,
            stop: body.get("stop_reason").and_then(Value::as_str).unwrap_or("").to_string(),
            usage: body.get("usage").map(Usage::from_body).unwrap_or_default(),
            nano: 0,
            millis,
            model: body.get("model").and_then(Value::as_str).unwrap_or("").to_string(),
            body,
        })
    }

    /// Turns a reply that is no use into a failure: a refusal, one cut off
    /// at the token limit, an empty one, or one that is not the JSON the
    /// schema asked for. With a schema it also reads the JSON.
    pub fn classify(&mut self, req: &Request) -> Result<(), Failure> {
        if self.stop == "refusal" {
            return Err(Failure::Refusal);
        }
        if self.stop == "max_tokens" {
            return Err(Failure::BadReply("the answer was cut off".into()));
        }
        if self.text.trim().is_empty() {
            return Err(Failure::BadReply("the answer was empty".into()));
        }
        if req.schema.is_some() {
            let v = serde_json::from_str::<Value>(self.text.trim())
                .map_err(|_| Failure::BadReply("the answer was not the JSON asked for".into()))?;
            self.json = Some(v);
        }
        Ok(())
    }
}

/// Every way a call can fail. All of them mean "play on the chips".
#[derive(Clone, Debug, PartialEq)]
pub enum Failure {
    NoKey,
    NoNetwork,
    Timeout,
    /// The API answered with an error (`status` 0 when there was none).
    Api { status: u16, message: String },
    Refusal,
    /// The account's credit or the key's own spending limit is used up.
    CreditGone,
    /// The game's own monthly cap is reached.
    CapReached,
    /// The API answered, but not with anything usable.
    BadReply(String),
}

impl Failure {
    /// A short stable name.
    pub fn kind(&self) -> &'static str {
        match self {
            Failure::NoKey => "no_key",
            Failure::NoNetwork => "no_network",
            Failure::Timeout => "timeout",
            Failure::Api { .. } => "api",
            Failure::Refusal => "refusal",
            Failure::CreditGone => "credit_gone",
            Failure::CapReached => "cap_reached",
            Failure::BadReply(_) => "bad_reply",
        }
    }

    /// The one plain line the screen shows.
    pub fn note(&self) -> String {
        match self {
            Failure::NoKey => "No key is set.".into(),
            Failure::NoNetwork => "The AI could not be reached.".into(),
            Failure::Timeout => "The AI took too long.".into(),
            Failure::Api { status: 401 | 403, .. } => "The key was not accepted.".into(),
            Failure::Api { status: 429 | 500..=599, .. } => "The AI is busy right now.".into(),
            Failure::Api { .. } => "The AI answered with an error.".into(),
            Failure::Refusal => "The AI would not answer that.".into(),
            Failure::CreditGone => "The API credit or the key's spending limit is used up.".into(),
            Failure::CapReached => "This month's $50 cap is reached.".into(),
            Failure::BadReply(_) => "The AI's answer could not be used.".into(),
        }
    }

    /// Failures that will repeat on every call until something changes
    /// (a new key, more credit): the boxes stay hidden until then.
    pub fn is_lasting(&self) -> bool {
        matches!(self, Failure::Api { status: 401 | 403, .. } | Failure::CreditGone)
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.note())
    }
}

/// Something that answers a request.
pub trait Model: Send + Sync {
    fn call(&self, req: &Request) -> Result<Reply, Failure>;

    /// True for the fake.
    fn is_fake(&self) -> bool {
        false
    }
}

/// The real client, on `ureq`. It holds the key and nothing in the game
/// can read it back.
pub struct Real {
    key: Key,
    base: String,
    timeout: Duration,
    proxy_from_env: bool,
    agent: ureq::Agent,
}

impl Real {
    pub fn new(key: Key) -> Real {
        Real::build(key, BASE_URL.to_string(), TIMEOUT, false)
    }

    /// For tests against a local server.
    pub fn with_base(self, base: &str) -> Real {
        Real::build(self.key, base.trim_end_matches('/').to_string(), self.timeout, self.proxy_from_env)
    }

    pub fn with_timeout(self, timeout: Duration) -> Real {
        Real::build(self.key, self.base, timeout, self.proxy_from_env)
    }

    /// For the real test on a thread machine only: goes through the proxy
    /// the environment names (`HTTPS_PROXY`), and trusts the CA bundle it
    /// names (`SSL_CERT_FILE`, read by the native certificate loader), with
    /// every TLS check left on. The game's own client never calls this.
    pub fn with_proxy_from_env(self) -> Real {
        Real::build(self.key, self.base, self.timeout, true)
    }

    fn build(key: Key, base: String, timeout: Duration, proxy_from_env: bool) -> Real {
        let agent = ureq::AgentBuilder::new().timeout(timeout).try_proxy_from_env(proxy_from_env).build();
        Real { key, base, timeout, proxy_from_env, agent }
    }
}

impl fmt::Debug for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Real({:?})", self.key)
    }
}

impl Model for Real {
    fn call(&self, req: &Request) -> Result<Reply, Failure> {
        let t = Instant::now();
        let sent = self
            .agent
            .post(&format!("{}/v1/messages", self.base))
            .set("x-api-key", self.key.expose())
            .set("anthropic-version", VERSION)
            .send_json(req.body());
        let millis = || t.elapsed().as_millis() as u64;
        match sent {
            Ok(resp) => {
                let body: Value = resp.into_json().map_err(|e| self.transport(&e.to_string()))?;
                Reply::from_body(body, millis())
            }
            Err(ureq::Error::Status(code, resp)) => {
                let text = resp.into_string().unwrap_or_default();
                Err(self.status_failure(code, &text))
            }
            Err(ureq::Error::Transport(tr)) => Err(self.transport(&tr.to_string())),
        }
    }
}

impl Real {
    /// A call that never got an answer: too slow, or not reached.
    fn transport(&self, what: &str) -> Failure {
        let w = what.to_lowercase();
        if w.contains("timed out") || w.contains("timeout") || w.contains("deadline") {
            Failure::Timeout
        } else {
            Failure::NoNetwork
        }
    }

    /// An HTTP error: the API's own message, with the key taken out of it.
    fn status_failure(&self, code: u16, text: &str) -> Failure {
        let v: Value = serde_json::from_str(text).unwrap_or(Value::Null);
        let message = self.key.scrub(v.pointer("/error/message").and_then(Value::as_str).unwrap_or(""));
        let m = message.to_lowercase();
        if m.contains("credit balance") || m.contains("usage limit") || m.contains("spend limit") || code == 402 {
            return Failure::CreditGone;
        }
        Failure::Api { status: code, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "sk-ant-api03-MARKERabcdefghijklmnopqrstuvwxyz0123456789-abcd";

    #[test]
    fn a_key_is_checked_and_never_shown_whole() {
        assert!(Key::parse("hello").is_err());
        assert!(Key::parse("sk-ant-short").is_err());
        assert!(Key::parse("sk-ant-api03-with a space inside it, long enough").is_err());
        let k = Key::parse(&format!("  {KEY}\n")).unwrap();
        assert_eq!(k.tail(), "abcd");
        let shown = format!("{k:?}");
        assert_eq!(shown, "Key(...abcd)");
        assert!(!shown.contains("MARKER"));
        assert_eq!(k.scrub(&format!("bad key {KEY} sent")), "bad key ... sent");
    }

    #[test]
    fn the_body_has_cached_rules_thinking_off_and_the_schema() {
        let schema = json!({"type": "object", "properties": {"a": {"type": "string"}}});
        let b = Request::new("reader", "RULES", "CARD").schema(schema.clone()).max_tokens(300).body();
        assert_eq!(b["model"], "claude-haiku-5-5");
        assert_eq!(b["thinking"]["type"], "disabled");
        assert_eq!(b["system"][0]["text"], "RULES");
        assert_eq!(b["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(b["messages"][0]["content"], "CARD");
        assert_eq!(b["max_tokens"], 300);
        assert_eq!(b["output_config"]["format"]["type"], "json_schema");
        assert_eq!(b["output_config"]["format"]["schema"], schema);
        let plain = Request::new("test", "R", "U").no_cache().body();
        assert!(plain["system"][0].get("cache_control").is_none());
        assert!(plain.get("output_config").is_none());
        // The label is the game's own note and is never sent.
        assert!(!plain.to_string().contains("test\""));
    }

    #[test]
    fn usage_reads_the_cache_split_when_there_is_one() {
        let plain = Usage::from_body(&json!({"input_tokens": 10, "output_tokens": 5, "cache_creation_input_tokens": 7}));
        assert_eq!((plain.input, plain.output, plain.cache_write_5m, plain.cache_write_1h), (10, 5, 7, 0));
        let split = Usage::from_body(&json!({
            "input_tokens": 1, "output_tokens": 2, "cache_read_input_tokens": 3, "cache_creation_input_tokens": 9,
            "cache_creation": {"ephemeral_5m_input_tokens": 4, "ephemeral_1h_input_tokens": 5}
        }));
        assert_eq!((split.cache_write_5m, split.cache_write_1h, split.cache_read), (4, 5, 3));
        assert_eq!(split.prompt(), 1 + 3 + 9);
    }

    #[test]
    fn useless_replies_become_failures_but_keep_their_usage() {
        let req = Request::new("t", "r", "u");
        let mk = |stop: &str, text: &str| {
            Reply::from_body(
                json!({"content": [{"type": "text", "text": text}], "stop_reason": stop, "usage": {"input_tokens": 4, "output_tokens": 2}}),
                1,
            )
            .unwrap()
        };
        assert_eq!(mk("refusal", "").classify(&req), Err(Failure::Refusal));
        assert!(matches!(mk("max_tokens", "half a sent").classify(&req), Err(Failure::BadReply(_))));
        assert!(matches!(mk("end_turn", "  ").classify(&req), Err(Failure::BadReply(_))));
        assert_eq!(mk("refusal", "").usage.input, 4);
        let ok = mk("end_turn", "fine");
        let mut ok = ok;
        assert!(ok.classify(&req).is_ok());
        let schema_req = Request::new("t", "r", "u").schema(json!({"type": "object"}));
        let mut bad = mk("end_turn", "not json");
        assert!(matches!(bad.classify(&schema_req), Err(Failure::BadReply(_))));
        let mut good = mk("end_turn", "{\"a\": 1}");
        good.classify(&schema_req).unwrap();
        assert_eq!(good.json.unwrap()["a"], 1);
    }

    #[test]
    fn every_failure_has_a_plain_line_and_the_lasting_ones_are_marked() {
        let all = [
            Failure::NoKey,
            Failure::NoNetwork,
            Failure::Timeout,
            Failure::Api { status: 401, message: String::new() },
            Failure::Api { status: 529, message: String::new() },
            Failure::Api { status: 400, message: String::new() },
            Failure::Refusal,
            Failure::CreditGone,
            Failure::CapReached,
            Failure::BadReply("x".into()),
        ];
        for f in &all {
            assert!(f.note().ends_with('.') && !f.kind().is_empty());
        }
        assert!(Failure::CreditGone.is_lasting());
        assert!(Failure::Api { status: 401, message: String::new() }.is_lasting());
        assert!(!Failure::Timeout.is_lasting() && !Failure::CapReached.is_lasting());
    }
}
