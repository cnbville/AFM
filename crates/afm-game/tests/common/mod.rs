//! Helpers shared by the game's test files (live talks, seam 7). Each file
//! under `tests/` is its own crate and uses only some of these, so unused
//! code is allowed here.
#![allow(dead_code)]

use serde_json::Value;

/// Keys that would carry something the game hides: a true potential or
/// ceiling, a true rating. The draft room's and the squad boards' lists,
/// merged.
pub const HIDDEN_KEYS: [&str; 6] = ["potential", "ceiling", "truth", "overall_true", "true", "pot"];

/// No hidden number anywhere in an answer: no hidden key at any depth, and
/// every staff read a range at least 2 wide. `cmd` and `path` name the place
/// in the panic.
pub fn nothing_hidden(cmd: &str, v: &Value, path: &str) {
    match v {
        Value::Object(o) => {
            for (k, x) in o {
                let p = format!("{path}.{k}");
                assert!(!HIDDEN_KEYS.contains(&k.as_str()), "{cmd}{p}");
                if k == "read"
                    && let Some(a) = x.as_array()
                    && let (Some(lo), Some(hi)) = (a.first().and_then(Value::as_f64), a.get(1).and_then(Value::as_f64))
                {
                    assert!(hi - lo >= 2.0, "{cmd}{p}: {x} is too sure to be a staff's read");
                }
                nothing_hidden(cmd, x, &p);
            }
        }
        Value::Array(a) => a.iter().enumerate().for_each(|(i, x)| nothing_hidden(cmd, x, &format!("{path}[{i}]"))),
        _ => {}
    }
}
