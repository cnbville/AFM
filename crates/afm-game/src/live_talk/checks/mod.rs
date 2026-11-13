//! The checks (live talks, the core): `ai_checks` runs sets of labelled
//! lines through the reader, one call after another on one ticket, and
//! reports for each set how many it read as labelled, the cost and the
//! seconds per line. The core ships the frame and LT18's set
//! (`staff.json`); lanes 7 and 9 fill `deals.json` and `one_on_one.json`.
//! A set's card is written out in its file, so a check needs no world.

use afm_ai::Request;
use serde::Deserialize;
use serde_json::{Value, json};

use super::card::{Card, Keys, Spoken, Tagged};
use super::say::{self, stand_in};
use super::tickets::{self, Next, Step};
use crate::{Game, Reply};

/// Every check set, in report order.
pub const SETS: [(&str, &str); 3] = [
    ("staff", include_str!("staff.json")),
    ("deals", include_str!("deals.json")),
    ("one_on_one", include_str!("one_on_one.json")),
];

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Set {
    pub name: String,
    pub card: SetCard,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetCard {
    pub setting: String,
    pub coach: String,
    pub them: String,
    pub so_far: Vec<SetSpoken>,
    pub options: Vec<SetOption>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetSpoken {
    pub who: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetOption {
    pub code: u32,
    pub text: String,
}

/// One labelled line: the option code it means, or "not_a_line".
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Line {
    pub text: String,
    pub want: Value,
}

/// Every set, read from its file. A set that doesn't parse is left out.
pub fn sets() -> Vec<Set> {
    SETS.iter().filter_map(|(_, raw)| serde_json::from_str(raw).ok()).collect()
}

impl Set {
    /// The set's card as the reader gets it, with its keys.
    pub fn card(&self) -> (Card, Keys) {
        let mut card = Card {
            setting: self.card.setting.clone(),
            coach: self.card.coach.clone(),
            them: self.card.them.clone(),
            so_far: self.card.so_far.iter().map(|s| Spoken { who: s.who.clone(), text: s.text.clone() }).collect(),
            ..Card::default()
        };
        let mut keys = Keys::default();
        for (k, o) in self.card.options.iter().enumerate() {
            let tag = format!("o{}", k + 1);
            card.options.push(Tagged { tag: tag.clone(), text: o.text.clone() });
            keys.options.push((tag, o.code));
        }
        (card, keys)
    }
}

/// Whether a reading matches a line's label.
pub fn matches(reading: &Value, keys: &Keys, want: &Value) -> bool {
    let fit = reading.get("fit").and_then(Value::as_str).unwrap_or("");
    let pick = reading.get("pick").and_then(Value::as_str).and_then(|t| keys.option(t));
    match want {
        Value::String(s) if s == "not_a_line" => fit == "not_a_line" || pick.is_none(),
        v => v.as_u64().is_some_and(|c| fit != "not_a_line" && pick == Some(c as u32)),
    }
}

struct Todo {
    set: usize,
    keys: Keys,
    want: Value,
    req: Request,
}

#[derive(Default)]
struct Tally {
    names: Vec<String>,
    score: Vec<u32>,
    of: Vec<u32>,
    nano: u64,
    millis: u64,
    lines: u32,
}

/// `ai_checks`: a ticket for the whole run.
pub fn ai_checks(g: &mut Game) -> Reply {
    let sets = sets();
    let mut tally = Tally::default();
    let mut todo = Vec::new();
    for (k, set) in sets.iter().enumerate() {
        tally.names.push(set.name.clone());
        tally.score.push(0);
        tally.of.push(0);
        let (card, keys) = set.card();
        for line in &set.lines {
            let req = Request::new(say::READER, say::READER_RULES, &card.reader_text(&line.text))
                .schema(say::reader_schema())
                .max_tokens(say::READER_TOKENS);
            if let Some(f) = g.ai.browser_fake() {
                f.push(say::READER, Ok(stand_in::reading(&card, &line.text)));
            }
            todo.push(Todo { set: k, keys: keys.clone(), want: line.want.clone(), req });
        }
    }
    todo.reverse();
    let Some(first) = todo.pop() else { return Err("No check lines.".into()) };
    let req = first.req.clone();
    tickets::start(g, req, step(first, todo, tally), false).map(|t| json!({ "ticket": t })).map_err(|f| f.note())
}

/// One line's answer, then the next line on the same ticket.
fn step(this: Todo, mut todo: Vec<Todo>, mut tally: Tally) -> Next {
    Box::new(move |_g, r| {
        let reply = match r {
            Ok(x) => x,
            Err(f) => return Step::off(&f),
        };
        tally.nano += reply.nano;
        tally.millis += reply.millis;
        tally.lines += 1;
        tally.of[this.set] += 1;
        if reply.json.as_ref().is_some_and(|v| matches(v, &this.keys, &this.want)) {
            tally.score[this.set] += 1;
        }
        match todo.pop() {
            Some(next) => {
                let req = next.req.clone();
                Step::Call(req, step(next, todo, tally))
            }
            None => Step::Done(report(&tally)),
        }
    })
}

fn report(t: &Tally) -> Value {
    let rows: Vec<Value> = (0..t.names.len())
        .filter(|&k| t.of[k] > 0)
        .map(|k| {
            json!({
                "name": t.names[k],
                "score": t.score[k],
                "of": t.of[k],
                "bar": f64::from(t.score[k]) / f64::from(t.of[k]),
            })
        })
        .collect();
    let lines = f64::from(t.lines.max(1));
    json!({
        "state": "done",
        "rows": rows,
        "cost": t.nano as f64 / 1e9,
        "seconds": (t.millis as f64 / lines / 10.0).round() / 100.0,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use afm_ai::Fake;
    use afm_ai::fake::body;
    use serde_json::json;

    use super::*;

    #[test]
    fn every_set_parses_and_staff_holds_lt18s_thirty_lines() {
        let sets = sets();
        assert_eq!(sets.len(), SETS.len(), "every file parses");
        let staff = &sets[0];
        assert_eq!((staff.name.as_str(), staff.lines.len()), ("staff", 30));
        let (card, keys) = staff.card();
        assert_eq!(card.options.len(), 5);
        for code in [60, 61, 62, 63, 69] {
            let n = staff.lines.iter().filter(|l| l.want == json!(code)).count();
            assert_eq!(n, 5, "{code}");
            assert!(keys.option_tag(code).is_some());
        }
        assert_eq!(staff.lines.iter().filter(|l| l.want == json!("not_a_line")).count(), 5);
        assert!(sets[1].lines.is_empty() && sets[2].lines.is_empty(), "lanes 7 and 9 fill theirs");
    }

    #[test]
    fn the_checks_run_on_the_fake_and_count_the_matches() {
        let home = std::env::temp_dir().join(format!("afm-live-checks-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut g = Game::new(home);
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        let staff = &sets()[0];
        let (_, keys) = staff.card();
        // Right on every line but the last two.
        for (k, line) in staff.lines.iter().enumerate() {
            let pick = match line.want.as_u64() {
                Some(c) if k < 28 => keys.option_tag(c as u32).unwrap().to_string(),
                Some(_) => "none".to_string(),
                None if k < 28 => "none".to_string(),
                None => "o1".to_string(),
            };
            let fit = if pick == "none" { "not_a_line" } else { "clear" };
            let v = json!({ "pick": pick, "also": "none", "tone": "calm", "promise": null, "claims": [], "named": [], "fit": fit });
            f.push(say::READER, Ok(body(&v.to_string(), 700, 60)));
        }
        let t = g.call("ai_checks", &json!({})).unwrap()["ticket"].as_u64().unwrap();
        let start = Instant::now();
        let v = loop {
            let v = g.call("ai_poll", &json!({ "ticket": t })).unwrap();
            if v["state"] != "wait" || start.elapsed() > Duration::from_secs(10) {
                break v;
            }
            std::thread::sleep(Duration::from_millis(2));
        };
        assert_eq!(v["state"], "done", "{v}");
        assert_eq!(v["rows"], json!([{ "name": "staff", "score": 28, "of": 30, "bar": 28.0 / 30.0 }]));
        assert!(v["cost"].as_f64().unwrap() > 0.0 && v["seconds"].is_number());
        assert_eq!(f.asked_count(), 30, "one call a line");
        // With no client there is no run.
        let mut h = Game::new(std::env::temp_dir().join(format!("afm-live-checks2-{}", std::process::id())));
        assert_eq!(h.call("ai_checks", &json!({})), Err("No key is set.".into()));
    }
}
