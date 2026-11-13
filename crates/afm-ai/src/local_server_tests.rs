//! The real client against a small server on this machine: what it sends,
//! and each way a call can fail. No network, no key of any value. These are
//! lib tests, so the quick check runs them (they take under a second).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::{Failure, Key, Model, Real, Request};
use serde_json::{Value, json};

const KEY: &str = "sk-ant-api03-LOCALTESTKEYabcdefghijklmnopqrstuvwxyz0123-wxyz";

/// One canned answer: status, body, and how long to wait before sending it.
struct Answer(u16, String, u64);

/// Serves the answers in order, one connection each, and keeps what it got.
fn serve(answers: Vec<Answer>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let seen = Arc::new(Mutex::new(Vec::new()));
    let log = seen.clone();
    std::thread::spawn(move || {
        for Answer(status, body, wait) in answers {
            let Ok((mut s, _)) = listener.accept() else { return };
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            // Read the head, then the body by its length.
            let (head_end, len) = loop {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break (buf.len(), 0);
                }
                buf.extend_from_slice(&chunk[..n]);
                if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf[..p]).to_lowercase();
                    let len = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    break (p + 4, len);
                }
            };
            while buf.len() < head_end + len {
                let n = s.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            log.lock().unwrap().push(String::from_utf8_lossy(&buf).to_string());
            std::thread::sleep(Duration::from_millis(wait));
            let reply = format!(
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(reply.as_bytes());
        }
    });
    (url, seen)
}

fn real(url: &str) -> Real {
    Real::new(Key::parse(KEY).unwrap()).with_base(url)
}

fn ok_body() -> String {
    json!({
        "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-haiku-5-5",
        "content": [{"type": "text", "text": "ok"}], "stop_reason": "end_turn",
        "usage": {"input_tokens": 20, "output_tokens": 2}
    })
    .to_string()
}

fn err_body(kind: &str, message: &str) -> String {
    json!({"type": "error", "error": {"type": kind, "message": message}}).to_string()
}

#[test]
fn a_call_sends_the_key_the_version_and_the_body_and_reads_the_reply() {
    let (url, seen) = serve(vec![Answer(200, ok_body(), 0)]);
    let req = Request::new("test", "Reply with ok.", "ping").no_cache();
    let reply = real(&url).call(&req).unwrap();
    assert_eq!(reply.text, "ok");
    assert_eq!(reply.usage.input, 20);
    let got = seen.lock().unwrap()[0].clone();
    let lower = got.to_lowercase();
    assert!(got.starts_with("POST /v1/messages "), "{got}");
    assert!(lower.contains(&format!("x-api-key: {}", KEY.to_lowercase())));
    assert!(lower.contains("anthropic-version: 2023-06-01"));
    let body: Value = serde_json::from_str(got.split("\r\n\r\n").nth(1).unwrap()).unwrap();
    assert_eq!(body, req.body());
    // The body is the request and nothing else: the key rides only in the header.
    assert!(!body.to_string().contains("LOCALTESTKEY"));
}

#[test]
fn each_http_error_becomes_the_right_failure() {
    let cases = [
        (401, err_body("authentication_error", "invalid x-api-key"), "api"),
        (429, err_body("rate_limit_error", "slow down"), "api"),
        (529, err_body("overloaded_error", "busy"), "api"),
        (400, err_body("invalid_request_error", "Your credit balance is too low to access the API."), "credit_gone"),
        (
            400,
            err_body("invalid_request_error", "You have reached your specified workspace API usage limits."),
            "credit_gone",
        ),
        (500, "not json at all".to_string(), "api"),
    ];
    for (status, body, kind) in cases {
        let (url, _) = serve(vec![Answer(status, body, 0)]);
        let err = real(&url).call(&Request::new("test", "r", "u")).unwrap_err();
        assert_eq!(err.kind(), kind, "status {status}: {err:?}");
        if let Failure::Api { status: s, .. } = err {
            assert_eq!(s, status);
        }
    }
}

#[test]
fn an_error_message_never_carries_the_key() {
    let (url, _) = serve(vec![Answer(401, err_body("authentication_error", &format!("bad key {KEY} given")), 0)]);
    let err = real(&url).call(&Request::new("test", "r", "u")).unwrap_err();
    let Failure::Api { message, .. } = &err else { panic!("{err:?}") };
    assert!(!message.contains("LOCALTESTKEY"), "{message}");
    assert!(!format!("{err:?}").contains("LOCALTESTKEY"));
}

#[test]
fn a_slow_server_times_out() {
    let (url, _) = serve(vec![Answer(200, ok_body(), 1500)]);
    let r = real(&url).with_timeout(Duration::from_millis(300));
    assert_eq!(r.call(&Request::new("test", "r", "u")).unwrap_err(), Failure::Timeout);
}

#[test]
fn nothing_listening_is_no_network() {
    // A port that was free a moment ago and has nothing on it.
    let url = {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        format!("http://{}", l.local_addr().unwrap())
    };
    assert_eq!(real(&url).call(&Request::new("test", "r", "u")).unwrap_err(), Failure::NoNetwork);
}

#[test]
fn a_reply_that_is_not_json_is_no_use() {
    let (url, _) = serve(vec![Answer(200, "<html>proxy page</html>".into(), 0)]);
    let err = real(&url).call(&Request::new("test", "r", "u")).unwrap_err();
    assert!(matches!(err, Failure::NoNetwork | Failure::Api { .. } | Failure::BadReply(_)), "{err:?}");
}

#[test]
fn the_debug_text_of_the_client_shows_only_the_tail() {
    let r = Real::new(Key::parse(KEY).unwrap());
    let shown = format!("{r:?}");
    assert_eq!(shown, "Real(Key(...wxyz))");
}
