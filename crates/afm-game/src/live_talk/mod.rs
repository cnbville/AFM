//! Live talks (the live AI update, lane 1: the pipe). Typed lines to your
//! players, recruits and staff, answered in character by Claude Haiku 5.5, on
//! top of the chips and the line bank, which always still work.
//!
//! This lane is the plumbing and nothing of the game: the key (in memory
//! only, `key.rs`), the client and its fake and the spend book with its $50
//! cap (the `afm-ai` crate), tickets that run a call beside the game
//! (`tickets.rs`), the commands below, the demo that answers every state of
//! the contract on the fake (`demo.rs`), and the one step after every command
//! (`after`). The core (lane 3) fills `after` and adds the typed-line
//! commands. The contract with the screens is in
//! `docs/reference/how-it-works/live-talks.md`.
//!
//! Commands: `ai_status`, `ai_key_set {key}`, `ai_key_clear`, `ai_test`,
//! `ai_poll {ticket}` and `ai_demo {state?}` (the fake only). The core
//! (lane 3) adds `people_say` (a typed line in any scene), `talk_typed`
//! (the pregame and halftime room), `fo_say` (contract talks, trade calls,
//! NIL forms), `ai_ask` (every ask box), `ai_checks`, and question 5's two
//! that need no key, `fo_fact` and `scout_source`. Each later lane fills its
//! own stub under `say/` and `ask/` and never edits this file.
//!
//! The rule under all of it: the AI only reads your words and writes theirs;
//! the game decides every result. No API call runs inside a command.

pub mod ask;
pub mod card;
pub mod checks;
pub mod demo;
pub mod filter;
pub mod key;
pub mod say;
pub mod tickets;

use afm_ai::Request;
use serde_json::{Value, json};

use crate::{Game, Reply};
use tickets::{Next, Step};

pub use key::Ai;

/// What a box that comes with a later lane answers until that lane fills
/// its stub (no flag shows it, so a screen never sends it).
pub const NOT_YET: &str = "That box comes with a later update.";

/// The game's lane for the live talks commands (`Game::run` asks each lane in
/// turn). Returns `None` for a command that is not its own.
pub fn call(g: &mut Game, cmd: &str, args: &Value) -> Option<Reply> {
    Some(match cmd {
        "ai_status" => Ok(g.ai.status()),
        "ai_key_set" => {
            let key = args.get("key").and_then(Value::as_str).unwrap_or("");
            g.ai.set_key(key).map(|_| g.ai.status())
        }
        "ai_key_clear" => {
            g.ai.clear_key();
            Ok(g.ai.status())
        }
        "ai_test" => test(g),
        "ai_poll" => match args.get("ticket").and_then(Value::as_u64) {
            Some(id) => Ok(tickets::poll(g, id)),
            None => Err("Missing ticket.".into()),
        },
        "ai_demo" => demo_cmd(g, args),
        "people_say" => say::people_say(g, args),
        "talk_typed" => say::rooms::typed(g, args),
        "fo_say" => say::fo_say(g, args),
        "ai_ask" => ask::ask(g, args),
        "ai_checks" => checks::ai_checks(g),
        "fo_fact" => say::money::fact(g, args),
        "scout_source" => ask::source::source_cmd(g, args),
        _ => return None,
    })
}

/// The one step after every command that worked, beside
/// `lane_handbook::after_call`. It may change the reply: question 5's lists,
/// which need no key, then, only when `g.ai.usable()`, the `say` box on a
/// scene that takes a typed line and each lane's own boxes (the shapes are
/// in the contract). A reply nested in a poll's answer gets its boxes the
/// same way.
pub fn after(g: &mut Game, cmd: &str, _args: &Value, reply: &mut Reply) {
    g.ai.tickets.prune();
    g.ai.waiting.prune();
    let Ok(v) = reply else { return };
    say::money::facts(g, cmd, v);
    say::recruit::facts(g, cmd, v);
    ask::source::sources(g, cmd, v);
    if !g.ai.usable() {
        return;
    }
    if cmd == "ai_poll" {
        if let Some(inner) = v.get_mut("reply").filter(|r| r.is_object()) {
            say::flag(g, cmd, inner);
            ask::flags(g, cmd, inner);
        }
        return;
    }
    say::flag(g, cmd, v);
    ask::flags(g, cmd, v);
}

/// The settings card's Test: one tiny call, to see that the key is accepted.
/// It goes through even after a lasting failure, since a good test is how that
/// is cleared.
fn test(g: &mut Game) -> Reply {
    let fake = g.ai.is_fake();
    let req = Request::new("test", "Reply with the single word ok.", "ping").no_cache().max_tokens(16);
    let next: Next = Box::new(move |g, r| match r {
        Ok(reply) => Step::Done(json!({
            "state": "done",
            "text": if fake { "The fake client answered." } else { "The key works." },
            "seconds": (reply.millis as f64 / 10.0).round() / 100.0,
            "cost": reply.nano as f64 / 1e9,
            "status": g.ai.status(),
        })),
        Err(f) => Step::off(&f),
    });
    tickets::start(g, req, next, true).map(|id| json!({ "ticket": id })).map_err(|f| f.note())
}

/// `ai_demo`: every state of the contract with set content, so a screen can
/// be built before the core exists. The fake only.
fn demo_cmd(g: &mut Game, args: &Value) -> Reply {
    if !g.ai.is_fake() {
        return Err("The demo answers only on the fake client. Start the browser build with AFM_AI_FAKE=1.".into());
    }
    let Some(state) = args.get("state").and_then(Value::as_str) else {
        return Ok(json!({ "states": demo::STATES, "flags": demo::flags() }));
    };
    let (answer, waits) = match state {
        // Waits until the ticket is dropped.
        "wait" => (json!({ "state": "wait" }), u32::MAX),
        s => (demo::answer(s).ok_or_else(|| format!("Unknown demo state: {s}"))?, demo::WAITS),
    };
    Ok(json!({ "ticket": tickets::start_demo(g, answer, waits) }))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use afm_ai::{Failure, Fake, Request};
    use serde_json::{Value, json};

    use super::tickets::{self, Next, Step};
    use crate::Game;

    const MARKER: &str = "sk-ant-api03-MARKERzyxwvutsrqponmlkjihgfedcba9876543210-abcd";

    fn home(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("afm-live-talk-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn game(name: &str) -> (Game, PathBuf) {
        let h = home(name);
        (Game::new(h.clone()), h)
    }

    fn ok(g: &mut Game, cmd: &str, args: Value) -> Value {
        g.call(cmd, &args).unwrap_or_else(|e| panic!("{cmd}: {e}"))
    }

    /// Polls until the ticket stops saying wait (2 seconds at most).
    fn finish(g: &mut Game, ticket: u64) -> Value {
        let t = Instant::now();
        loop {
            let v = ok(g, "ai_poll", json!({ "ticket": ticket }));
            if v["state"] != "wait" || t.elapsed() > Duration::from_secs(2) {
                return v;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn fake_game(name: &str) -> (Game, Arc<Fake>, PathBuf) {
        let (mut g, h) = game(name);
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        (g, f, h)
    }

    /// Every file under `dir` that holds `needle`.
    fn files_holding(dir: &Path, needle: &str) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut todo = vec![dir.to_path_buf()];
        while let Some(d) = todo.pop() {
            for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    todo.push(p);
                } else if std::fs::read(&p).is_ok_and(|b| b.windows(needle.len()).any(|w| w == needle.as_bytes())) {
                    found.push(p);
                }
            }
        }
        found
    }

    #[test]
    fn a_game_with_no_key_builds_no_client() {
        let (mut g, _) = game("nokey");
        assert!(!g.ai.has_client() && !g.ai.has_real_client() && !g.ai.is_fake());
        let s = ok(&mut g, "ai_status", json!({}));
        assert_eq!(s["key_set"], false);
        assert_eq!(s["usable"], false);
        assert_eq!(s["cap"], 50.0);
        assert_eq!(s["spent"], 0.0);
        assert_eq!(g.call("ai_test", &json!({})), Err("No key is set.".into()));
        // The demo is for the fake only.
        assert!(g.call("ai_demo", &json!({})).is_err());
        // Clearing a key that is not there is fine.
        assert_eq!(ok(&mut g, "ai_key_clear", json!({}))["key_set"], false);
        assert!(!g.ai.has_client());
    }

    #[test]
    fn a_key_lives_in_memory_and_shows_only_its_tail() {
        let (mut g, _) = game("tail");
        assert!(g.call("ai_key_set", &json!({ "key": "not a key" })).is_err());
        assert!(g.call("ai_key_set", &json!({})).is_err());
        assert!(!g.ai.has_client(), "a bad key builds nothing");
        let s = ok(&mut g, "ai_key_set", json!({ "key": format!(" {MARKER}\n") }));
        assert_eq!(s["key_ending"], "...abcd");
        assert_eq!(s["key_set"], true);
        assert!(g.ai.has_real_client(), "a key builds the real client, which makes no call until asked");
        assert!(!s.to_string().contains("MARKER"));
        assert!(!format!("{:?}", g.ai).contains("MARKER"));
        assert!(format!("{:?}", g.ai).contains("...abcd"));
        let cleared = ok(&mut g, "ai_key_clear", json!({}));
        assert_eq!(cleared["key_set"], false);
        assert!(cleared["key_ending"].is_null());
        assert!(!g.ai.has_client());
    }

    #[test]
    fn the_key_reaches_no_file_of_the_game_folder() {
        let (mut g, h) = game("leak");
        ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
        // Every command that runs with no world loaded. The real client is
        // built but none of these calls the API (ai_test waits for the fake below).
        for cmd in [
            "hello",
            "library",
            "job",
            "job_stop",
            "status",
            "card",
            "home",
            "inbox",
            "setup",
            "save",
            "bug",
            "ai_status",
            "ai_demo",
            "ai_poll",
            "ai_key_set",
            "ai_key_clear",
            "ai_key_set",
        ] {
            let args = if cmd == "ai_key_set" { json!({ "key": MARKER }) } else { json!({ "ticket": 1 }) };
            let _ = g.call(cmd, &args);
        }
        assert_eq!(files_holding(&h, "MARKER"), Vec::<PathBuf>::new());
        // With the fake, so no call leaves, a test call writes the spend book.
        g.ai.use_fake(Arc::new(Fake::new()));
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        assert_eq!(finish(&mut g, id)["state"], "done");
        assert!(h.join("ai-spend-fake.json").exists());
        assert_eq!(files_holding(&h, "MARKER"), Vec::<PathBuf>::new());
    }

    #[test]
    fn a_test_call_runs_on_a_worker_and_is_counted() {
        let (mut g, fake, h) = fake_game("flow");
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        let v = finish(&mut g, id);
        assert_eq!(v["state"], "done");
        assert_eq!(v["text"], "The fake client answered.");
        assert_eq!(fake.asked_count(), 1);
        let s = ok(&mut g, "ai_status", json!({}));
        assert_eq!(s["calls"], 1);
        assert!(s["spent"].as_f64().unwrap() > 0.0 && s["fake"] == true);
        // The fake's spend never goes in the real month's book.
        assert!(!h.join("ai-spend.json").exists());
        // A ticket is answered once.
        assert_eq!(ok(&mut g, "ai_poll", json!({ "ticket": id }))["state"], "off");
    }

    #[test]
    fn a_slow_call_answers_wait_and_does_not_hold_the_game() {
        let (mut g, _) = game("slow");
        g.ai.use_fake(Arc::new(Fake::for_browser()));
        let t = Instant::now();
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        assert!(t.elapsed() < Duration::from_millis(300), "the command returns at once");
        assert_eq!(ok(&mut g, "ai_poll", json!({ "ticket": id }))["state"], "wait");
        // Other commands answer meanwhile.
        assert_eq!(ok(&mut g, "hello", json!({}))["game"], "AFM");
        assert_eq!(finish(&mut g, id)["state"], "done");
    }

    #[test]
    fn a_ticket_can_chain_a_second_call() {
        let (mut g, fake, _) = fake_game("chain");
        // The reader, then the game's step, then the writer: the core's shape.
        let next: Next = Box::new(|_g, r| {
            let _reading = r.unwrap();
            let writer: Next = Box::new(|_g, r| Step::Done(json!({ "state": "done", "words": r.unwrap().text })));
            Step::Call(Request::new("writer", "rules", "card"), writer)
        });
        let id = tickets::start(&mut g, Request::new("reader", "rules", "card"), next, false).unwrap();
        let v = finish(&mut g, id);
        assert_eq!(v["state"], "done");
        assert!(v["words"].as_str().unwrap().contains("thirty games"));
        let labels: Vec<String> = fake.asked().into_iter().map(|r| r.label).collect();
        assert_eq!(labels, ["reader", "writer"]);
        assert_eq!(ok(&mut g, "ai_status", json!({}))["calls"], 2);
    }

    #[test]
    fn every_failure_kind_ends_in_off_with_a_plain_line() {
        let (mut g, fake, _) = fake_game("fails");
        for f in [
            Failure::NoNetwork,
            Failure::Timeout,
            Failure::Api { status: 500, message: "x".into() },
            Failure::Refusal,
            Failure::BadReply("x".into()),
        ] {
            fake.push("test", Err(f.clone()));
            let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
            let v = finish(&mut g, id);
            assert_eq!((v["state"].as_str(), v["note"].as_str()), (Some("off"), Some(f.note().as_str())), "{f:?}");
            assert!(g.ai.usable(), "{f:?} does not stand in the way of the next call");
        }
        // Fake mode needs no key for the boxes.
        assert_eq!(ok(&mut g, "ai_status", json!({}))["usable"], true);
    }

    #[test]
    fn a_lasting_failure_hides_the_boxes_until_a_good_test() {
        let (mut g, fake, _) = fake_game("lasting");
        for f in [Failure::CreditGone, Failure::Api { status: 401, message: String::new() }] {
            fake.push("test", Err(f.clone()));
            let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
            assert_eq!(finish(&mut g, id)["state"], "off");
            assert!(!g.ai.usable());
            let s = ok(&mut g, "ai_status", json!({}));
            assert_eq!(s["usable"], false);
            assert_eq!(s["note"], f.note());
            // Another typed line would not even be started...
            let blocked = tickets::start(
                &mut g,
                Request::new("writer", "r", "c"),
                Box::new(|_g, _r| Step::Done(Value::Null)),
                false,
            );
            assert_eq!(blocked.err(), Some(f.clone()));
            // ...but Test goes through, and a good one clears it.
            let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
            assert_eq!(finish(&mut g, id)["state"], "done");
            assert!(g.ai.usable());
        }
        // A new key clears it too.
        fake.push("test", Err(Failure::CreditGone));
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        finish(&mut g, id);
        assert!(!g.ai.usable());
        ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
        assert!(g.ai.usable());
    }

    #[test]
    fn a_late_answer_from_an_old_key_does_not_mark_the_new_one() {
        let (mut g, fake, _) = fake_game("stale");
        fake.push("test", Err(Failure::Api { status: 401, message: String::new() }));
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        // The fake answers at once; wait until the worker has left its answer.
        let t = Instant::now();
        while fake.asked_count() == 0 && t.elapsed() < Duration::from_secs(2) {
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(50));
        // A new key is pasted before the old answer is read.
        ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
        assert_eq!(finish(&mut g, id)["state"], "off", "the old answer still reaches its step");
        assert!(g.ai.usable(), "the new key is not marked refused by the old one's answer");
        assert_eq!(ok(&mut g, "ai_status", json!({}))["note"], Value::Null);
        // The same answer under the key in use does block.
        fake.push("test", Err(Failure::Api { status: 401, message: String::new() }));
        let id = ok(&mut g, "ai_test", json!({}))["ticket"].as_u64().unwrap();
        finish(&mut g, id);
        assert!(!g.ai.usable());
    }

    #[test]
    fn a_new_key_is_in_memory_until_the_app_says_it_saved_it() {
        let (mut g, _) = game("kept");
        ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
        assert_eq!(ok(&mut g, "ai_status", json!({}))["kept"], "memory");
        g.ai.set_kept("keychain");
        assert_eq!(ok(&mut g, "ai_status", json!({}))["kept"], "keychain");
        // A second key the Keychain then refuses must not read as kept there.
        let s = ok(&mut g, "ai_key_set", json!({ "key": MARKER.replace("abcd", "wxyz") }));
        assert_eq!(s["kept"], "memory");
        g.ai.set_kept("keychain");
        let cleared = ok(&mut g, "ai_key_clear", json!({}));
        assert!(cleared["kept"].is_null());
    }

    #[test]
    fn at_the_cap_everything_goes_back_to_the_chips() {
        let h = home("cap");
        let month = afm_ai::month_now();
        let book = json!({ "version": 1, "months": {
            "2000-01": { "nano": 50_000_000_000u64, "calls": 9 },
            month.clone(): { "nano": 50_000_000_000u64, "calls": 3 },
        }});
        std::fs::write(h.join("ai-spend.json"), book.to_string()).unwrap();
        let mut g = Game::new(h);
        ok(&mut g, "ai_key_set", json!({ "key": MARKER }));
        let s = ok(&mut g, "ai_status", json!({}));
        assert_eq!((s["cap_reached"].clone(), s["usable"].clone()), (json!(true), json!(false)));
        assert_eq!(s["spent"], 50.0);
        assert_eq!(s["note"], "This month's $50 cap is reached.");
        assert_eq!(g.call("ai_test", &json!({})), Err("This month's $50 cap is reached.".into()));
        // The old month's cap does not follow into a new month.
        let h2 = home("cap2");
        std::fs::write(
            h2.join("ai-spend.json"),
            json!({ "version": 1, "months": { "2000-01": { "nano": 50_000_000_000u64 } } }).to_string(),
        )
        .unwrap();
        let mut g2 = Game::new(h2);
        ok(&mut g2, "ai_key_set", json!({ "key": MARKER }));
        assert_eq!(ok(&mut g2, "ai_status", json!({}))["usable"], true);
    }

    #[test]
    fn too_many_open_tickets_are_refused() {
        let (mut g, _) = game("many");
        g.ai.use_fake(Arc::new(Fake::for_browser()));
        for _ in 0..tickets::MAX_OPEN {
            ok(&mut g, "ai_test", json!({}));
        }
        assert!(g.call("ai_test", &json!({})).unwrap_err().starts_with("Too many questions"));
    }

    #[test]
    fn the_demo_answers_every_state_of_the_contract() {
        let (mut g, _, _) = fake_game("demo");
        let list = ok(&mut g, "ai_demo", json!({}));
        let flags = &list["flags"];
        assert_eq!(flags["say"]["cmd"], "people_say");
        assert_eq!(flags["ask"].as_array().unwrap().len(), 4);
        assert_eq!(flags["ask"][3]["auto"], true);
        assert!(flags["facts"]["list"].is_array() && flags["sources"]["topics"].is_array());
        for s in list["states"].as_array().unwrap() {
            let s = s.as_str().unwrap();
            let id = ok(&mut g, "ai_demo", json!({ "state": s }))["ticket"].as_u64().unwrap();
            // Two waits first, as a slow call would give.
            assert_eq!(ok(&mut g, "ai_poll", json!({ "ticket": id }))["state"], "wait", "{s}");
            if s == "wait" {
                continue;
            }
            assert_eq!(ok(&mut g, "ai_poll", json!({ "ticket": id }))["state"], "wait", "{s}");
            let v = ok(&mut g, "ai_poll", json!({ "ticket": id }));
            assert!(["unsure", "done", "none", "off"].contains(&v["state"].as_str().unwrap()), "{s}: {v}");
        }
        assert!(g.call("ai_demo", &json!({ "state": "nonsense" })).is_err());
        // The answers the contract names, with their fields.
        let get = |g: &mut Game, s: &str| {
            let id = ok(g, "ai_demo", json!({ "state": s }))["ticket"].as_u64().unwrap();
            finish(g, id)
        };
        assert_eq!(get(&mut g, "unsure")["readings"][0]["pick"], 3);
        assert_eq!(get(&mut g, "done")["marks"][0]["mark"], "true");
        assert!(get(&mut g, "table")["table"]["rows"].is_array());
        assert_eq!(get(&mut g, "reload")["reload"], true);
        assert!(get(&mut g, "checks")["rows"][0]["bar"].is_number());
    }

    #[test]
    fn after_leaves_a_reply_as_it_found_it() {
        let (mut g, _, _) = fake_game("after");
        let hello = ok(&mut g, "hello", json!({}));
        assert!(hello.get("say").is_none() && hello.get("ask").is_none(), "lane 1 adds no box");
        let mut r = Ok(json!({ "a": 1 }));
        super::after(&mut g, "hello", &json!({}), &mut r);
        assert_eq!(r, Ok(json!({ "a": 1 })));
    }

    #[test]
    fn a_ticket_nobody_asks_for_is_dropped() {
        let (mut g, _, _) = fake_game("prune");
        ok(&mut g, "ai_demo", json!({ "state": "wait" }));
        assert_eq!(g.ai.tickets.len(), 1);
        g.ai.tickets.prune();
        assert_eq!(g.ai.tickets.len(), 1, "a young ticket stays");
        g.ai.tickets.prune_older_than(Duration::ZERO);
        assert!(g.ai.tickets.is_empty());
        assert_eq!(ok(&mut g, "ai_poll", json!({ "ticket": 1 }))["note"], "That answer is gone.");
    }
}
