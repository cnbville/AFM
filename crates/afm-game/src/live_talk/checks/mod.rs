//! The checks (live talks, the core): `ai_checks` runs sets of labelled
//! lines through the reader, one call after another on one ticket, and
//! reports for each set how many it read as labelled, the cost and the
//! seconds per line. The core ships the frame and LT18's set
//! (`staff.json`); lanes 7 and 9 fill `deals.json` and `one_on_one.json`.
//! A set's card is written out in its file, so a check needs no world.

use std::sync::Arc;

use afm_ai::Request;
use serde::Deserialize;
use serde_json::{Value, json};

use super::card::{Card, Keys, Spoken, Tagged};
use super::persona::{ear as persona_ear, voice as persona_voice};
use super::say::{self, stand_in};
use super::tickets::{self, Next, Step};
use crate::{Game, Reply};

pub mod ear;
pub mod voice;

/// Every reader check set, in report order.
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
    /// Lines meant to trick the reader or the game (lane 9's 40): reported
    /// as their own row, "<name> exploits", on the same card.
    pub exploits: Vec<Line>,
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
    /// Its tag on the card: o1, o2... by default; a one-on-one's turns are
    /// t1, t2...
    pub tag: String,
}

/// One labelled line: the option code it means, or "not_a_line". An
/// exploit line also carries what it tries (`why`), the move and tone it
/// would land as, and the facts a right reading lists (`claims`, the
/// reader's own form), so a test can land it on a real talk.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Line {
    pub text: String,
    pub want: Value,
    pub why: String,
    pub tone: String,
    pub claims: Vec<Value>,
    pub promise: Value,
}

/// Every set, read from its file, a set's exploits as a set of their own.
/// A set that doesn't parse is left out.
pub fn sets() -> Vec<Set> {
    let mut out = Vec::new();
    for (_, raw) in SETS {
        let Ok(mut set) = serde_json::from_str::<Set>(raw) else { continue };
        let exploits = std::mem::take(&mut set.exploits);
        let more = (!exploits.is_empty()).then(|| Set {
            name: format!("{} exploits", set.name),
            card: set.card.clone(),
            lines: exploits,
            exploits: Vec::new(),
        });
        out.push(set);
        out.extend(more);
    }
    out
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
            let tag = if o.tag.is_empty() { format!("o{}", k + 1) } else { o.tag.clone() };
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

/// What one line is graded against.
enum Check {
    /// A reader set's line: the option it means.
    Reader { keys: Keys, want: Value },
    /// The ear's line: what a right reading lists.
    Ear { set: Arc<ear::EarSet>, line: Arc<ear::EarLine> },
    /// The voice's line: the card he was given and what must and mustn't show.
    Voice { card: Arc<String>, line: Arc<voice::VoiceLine> },
}

struct Todo {
    set: usize,
    check: Check,
    req: Request,
    /// The line in words, for a miss.
    text: String,
    /// Whether its call already failed once (it is tried again once).
    retried: bool,
}

#[derive(Default)]
struct Tally {
    names: Vec<String>,
    /// The share of a set's lines that should pass (the tick on the card).
    bars: Vec<f64>,
    score: Vec<u32>,
    of: Vec<u32>,
    nano: Vec<u64>,
    millis: Vec<u64>,
    /// Lines that missed: the set, the line and why.
    misses: Vec<(String, String, String)>,
    /// Lines whose call failed twice: never graded, counted apart.
    not_run: Vec<u32>,
    /// Those lines: the set, the line and why the call failed.
    skipped: Vec<(String, String, String)>,
    model: String,
    /// Lines not run one after another.
    failed_in_row: u32,
}

/// Lines not run in a row that end a run (the network is down, the API is
/// out). A failed call is tried again once; a line whose call fails twice
/// is "not run", never a miss (the ear audit's item 5).
pub const MAX_FAILED_IN_ROW: u32 = 3;

impl Tally {
    fn add(&mut self, name: &str, bar: f64) -> usize {
        self.names.push(name.to_string());
        self.bars.push(bar);
        self.score.push(0);
        self.of.push(0);
        self.nano.push(0);
        self.millis.push(0);
        self.not_run.push(0);
        self.names.len() - 1
    }
}

/// The share of a reader set's lines that should pass (lane 9: bar 90,
/// proposed, not confirmed).
pub const READER_BAR: f64 = 0.90;

/// What `ai_checks` runs: `only` is "reader" (the older sets), "ear" (the
/// facets' labelled lines and exploits), "voice" (the leak baits) or
/// everything; `model` runs the same lines on another model to compare.
#[derive(Default)]
struct Args {
    only: String,
    model: Option<String>,
}

impl Args {
    fn from(v: &Value) -> Result<Args, String> {
        let only = v.get("only").and_then(Value::as_str).unwrap_or("all").to_string();
        if !["all", "reader", "ear", "voice"].contains(&only.as_str()) {
            return Err("only is all, reader, ear or voice.".into());
        }
        let model = v.get("model").and_then(Value::as_str).filter(|m| !m.is_empty()).map(String::from);
        if let Some(m) = &model
            && afm_ai::Price::of(m).is_none()
        {
            return Err(format!("There is no price for {m}."));
        }
        Ok(Args { only, model })
    }
    fn runs(&self, part: &str) -> bool {
        self.only == "all" || self.only == part
    }
}

/// `ai_checks {only?, model?}`: a ticket for the whole run.
pub fn ai_checks(g: &mut Game, args: &Value) -> Reply {
    let args = Args::from(args)?;
    let mut tally =
        Tally { model: args.model.clone().unwrap_or_else(|| afm_ai::MODEL.to_string()), ..Tally::default() };
    let mut todo: Vec<Todo> = Vec::new();
    let fake = g.ai.browser_fake();
    let finish = |req: Request| match &args.model {
        Some(m) => req.model(m),
        None => req,
    };
    if args.runs("reader") {
        for set in sets() {
            let bar = if set.name.ends_with("exploits") { ear::EXPLOIT_BAR } else { READER_BAR };
            let k = tally.add(&set.name, bar);
            let (card, keys) = set.card();
            // A one-on-one's sets read on lane 9's own rules.
            let (rules, schema) = if set.name.starts_with("one_on_one") {
                (say::one_on_one::RULES.as_str(), say::one_on_one::schema())
            } else {
                (say::READER_RULES.as_str(), say::reader_schema())
            };
            for line in &set.lines {
                let req = Request::new(say::READER, rules, &card.reader_text(&line.text))
                    .schema(schema.clone())
                    .max_tokens(say::READER_TOKENS);
                if let Some(f) = &fake {
                    f.push(say::READER, Ok(stand_in::reading(&card, &line.text)));
                }
                todo.push(Todo {
                    set: k,
                    check: Check::Reader { keys: keys.clone(), want: line.want.clone() },
                    req: finish(req),
                    text: line.text.clone(),
                    retried: false,
                });
            }
        }
    }
    if args.runs("ear") {
        // The ear's call as the game sends it (`persona::ear::request`):
        // the same rules, form, thinking and token limit.
        let ear_line = |todo: &mut Vec<Todo>, k: usize, set: &Arc<ear::EarSet>, line: &ear::EarLine| {
            let req = persona_ear::request(&set.body(line));
            if let Some(f) = &fake {
                let quiet = json!({ "acts": [], "hits": [], "dodged": false, "not_talk": false });
                f.push(persona_ear::EAR, Ok(afm_ai::fake::body(&quiet.to_string(), 900, 60)));
            }
            todo.push(Todo {
                set: k,
                check: Check::Ear { set: set.clone(), line: Arc::new(line.clone()) },
                req: finish(req),
                text: line.text.clone(),
                retried: false,
            });
        };
        for set in ear::sets() {
            let set = Arc::new(set);
            let plain = tally.add(&format!("ear: {}", set.name), ear::BAR);
            let exploits = tally.add(&format!("ear: {} exploits", set.name), ear::EXPLOIT_BAR);
            for (k, lines) in [(plain, &set.lines), (exploits, &set.exploits)] {
                for line in lines {
                    ear_line(&mut todo, k, &set, line);
                }
            }
        }
        // Lines on the same cards that no rule was written against (ear
        // audit item 6): the ear on wording it has never seen.
        let plain = tally.add("ear: unseen lines", ear::BAR);
        let exploits = tally.add("ear: unseen exploits", ear::EXPLOIT_BAR);
        for set in ear::unseen() {
            let set = Arc::new(set);
            for (k, lines) in [(plain, &set.lines), (exploits, &set.exploits)] {
                for line in lines {
                    ear_line(&mut todo, k, &set, line);
                }
            }
        }
    }
    if args.runs("voice") {
        let k = tally.add("voice leaks", voice::BAR);
        for case in voice::cases() {
            for line in &case.lines {
                let card = case.card.card(&line.text).text();
                let req = Request::new(persona_voice::VOICE, persona_voice::VOICE_RULES, &card)
                    .schema(persona_voice::schema())
                    .max_tokens(persona_voice::VOICE_TOKENS);
                if let Some(f) = &fake {
                    let said = json!({ "line": "I hear you, Coach.", "colour": "" });
                    f.push(persona_voice::VOICE, Ok(afm_ai::fake::body(&said.to_string(), 600, 40)));
                }
                todo.push(Todo {
                    set: k,
                    check: Check::Voice { card: Arc::new(card), line: Arc::new(line.clone()) },
                    req: finish(req),
                    text: line.text.clone(),
                    retried: false,
                });
            }
        }
    }
    todo.reverse();
    let Some(first) = todo.pop() else { return Err("No check lines.".into()) };
    let req = first.req.clone();
    tickets::start(g, req, step(first, todo, tally), false).map(|t| json!({ "ticket": t })).map_err(|f| f.note())
}

/// One line's answer, then the next line on the same ticket.
fn step(mut this: Todo, todo: Vec<Todo>, mut tally: Tally) -> Next {
    Box::new(move |_g, r| {
        // One line's failed call (a timeout, a refusal, a reply that won't
        // read) is tried again once; failing twice, the line is not run,
        // never a miss, and the run goes on. A lasting failure (the key,
        // the credit, the cap) or a few lines not run in a row end it.
        let reply = match r {
            Ok(x) => {
                tally.failed_in_row = 0;
                x
            }
            Err(f) => {
                if !f.is_retryable() {
                    return Step::off(&f);
                }
                if !this.retried {
                    this.retried = true;
                    let req = this.req.clone();
                    return Step::Call(req, step(this, todo, tally));
                }
                tally.failed_in_row += 1;
                if tally.failed_in_row >= MAX_FAILED_IN_ROW {
                    return Step::off(&f);
                }
                let note = match &f {
                    afm_ai::Failure::BadReply(m) => m.clone(),
                    _ => f.note(),
                };
                tally.not_run[this.set] += 1;
                tally.skipped.push((tally.names[this.set].clone(), this.text.clone(), note));
                return next(todo, tally);
            }
        };
        tally.nano[this.set] += reply.nano;
        tally.millis[this.set] += reply.millis;
        tally.of[this.set] += 1;
        let verdict = match (&this.check, reply.json.as_ref()) {
            (_, None) => Err("no JSON came back".to_string()),
            (Check::Reader { keys, want }, Some(v)) => {
                if matches(v, keys, want) {
                    Ok(())
                } else {
                    Err("read it as another option".to_string())
                }
            }
            (Check::Ear { set, line }, Some(v)) => ear::grade(v, set, line),
            (Check::Voice { card, line }, Some(v)) => voice::grade(v, card, line),
        };
        match verdict {
            Ok(()) => tally.score[this.set] += 1,
            Err(why) => {
                if tally.misses.len() < MAX_MISSES {
                    tally.misses.push((tally.names[this.set].clone(), this.text.clone(), why));
                }
            }
        }
        next(todo, tally)
    })
}

/// The next line's call, or the report.
fn next(mut todo: Vec<Todo>, tally: Tally) -> Step {
    match todo.pop() {
        Some(next) => {
            let req = next.req.clone();
            Step::Call(req, step(next, todo, tally))
        }
        None => Step::Done(report(&tally)),
    }
}

/// The most missed lines the report lists.
pub const MAX_MISSES: usize = 40;

fn report(t: &Tally) -> Value {
    let rows: Vec<Value> = (0..t.names.len())
        .filter(|&k| t.of[k] + t.not_run[k] > 0)
        .map(|k| {
            let of = f64::from(t.of[k].max(1));
            json!({
                "name": t.names[k],
                "score": t.score[k],
                "of": t.of[k],
                // Lines whose call failed twice: not in the score.
                "not_run": t.not_run[k],
                // The tick: how many lines should pass (the screen draws
                // the row amber under it).
                "bar": (of * t.bars[k]).ceil() as u32,
                "rate": f64::from(t.score[k]) / of,
                "cost": t.nano[k] as f64 / 1e9,
                "seconds": (t.millis[k] as f64 / of / 10.0).round() / 100.0,
            })
        })
        .collect();
    let nano: u64 = t.nano.iter().sum();
    let millis: u64 = t.millis.iter().sum();
    let lines = f64::from(t.of.iter().sum::<u32>().max(1));
    json!({
        "state": "done",
        "model": t.model,
        "rows": rows,
        "misses": t.misses.iter().map(|(set, text, why)| json!({ "set": set, "text": text, "why": why })).collect::<Vec<_>>(),
        "not_run": t.skipped.iter().map(|(set, text, why)| json!({ "set": set, "text": text, "why": why })).collect::<Vec<_>>(),
        "calls": t.of.iter().sum::<u32>(),
        "cost": nano as f64 / 1e9,
        "seconds": (millis as f64 / lines / 10.0).round() / 100.0,
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
    use crate::live_talk::persona::{ear as persona_ear, voice as persona_voice};

    #[test]
    fn every_set_parses_and_staff_holds_lt18s_thirty_lines() {
        let sets = sets();
        // Every file parses; the one-on-one's exploits are a row of their own.
        assert_eq!(sets.len(), SETS.len() + 1, "every file parses");
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
        assert!(sets[1].lines.is_empty(), "lane 7 fills its own");
    }

    #[test]
    fn the_one_on_one_sets_hold_a_hundred_lines_and_forty_exploits() {
        let sets = sets();
        let set = sets.iter().find(|s| s.name == "one_on_one").unwrap();
        let ex = sets.iter().find(|s| s.name == "one_on_one exploits").unwrap();
        assert_eq!((set.lines.len(), ex.lines.len()), (100, 40));
        let (card, keys) = set.card();
        // Every label is on the card, and the turns carry their own tags.
        for l in set.lines.iter().chain(&ex.lines) {
            if let Some(c) = l.want.as_u64() {
                assert!(keys.option_tag(c as u32).is_some(), "{}", l.text);
            } else {
                assert_eq!(l.want, json!("not_a_line"), "{}", l.text);
            }
            assert!(l.text.chars().count() <= 400);
        }
        assert!(card.options.iter().any(|o| o.tag == "t1") && card.options.iter().any(|o| o.tag == "o6"));
        // Each option has lines, and a fair share don't count as a line.
        for (_, code) in &keys.options {
            assert!(set.lines.iter().any(|l| l.want == json!(code)), "{code}");
        }
        let none = set.lines.iter().filter(|l| l.want == json!("not_a_line")).count();
        assert!((10..=15).contains(&none), "{none}");
        // The card reads on lane 9's schema: every tag is in its enum.
        let schema = crate::live_talk::say::one_on_one::schema();
        let tags: Vec<&str> =
            schema["properties"]["pick"]["enum"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
        for (t, _) in &keys.options {
            assert!(tags.contains(&t.as_str()), "{t}");
        }
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
        // The other sets' lines, each read as no line at all.
        let rest: usize = sets().iter().skip(1).map(|s| s.lines.len()).sum();
        for _ in 0..rest {
            let v = json!({ "pick": "none", "also": "none", "tone": "calm", "promise": null, "claims": [], "named": [], "fit": "not_a_line" });
            f.push(say::READER, Ok(body(&v.to_string(), 700, 60)));
        }
        let t = g.call("ai_checks", &json!({ "only": "reader" })).unwrap()["ticket"].as_u64().unwrap();
        let start = Instant::now();
        let v = loop {
            let v = g.call("ai_poll", &json!({ "ticket": t })).unwrap();
            if v["state"] != "wait" || start.elapsed() > Duration::from_secs(10) {
                break v;
            }
            std::thread::sleep(Duration::from_millis(2));
        };
        assert_eq!(v["state"], "done", "{v}");
        let staff = &v["rows"][0];
        assert_eq!(
            (staff["name"].as_str(), staff["score"].as_u64(), staff["of"].as_u64()),
            (Some("staff"), Some(28), Some(30))
        );
        // The tick is the count that should pass (90% of 30), not the score.
        assert_eq!(staff["bar"], 27);
        assert!((staff["rate"].as_f64().unwrap() - 28.0 / 30.0).abs() < 1e-9);
        assert!(staff["cost"].as_f64().unwrap() > 0.0 && staff["seconds"].is_number());
        assert_eq!(v["model"], "claude-haiku-5-5");
        // Lane 9's rows: only its not-a-line labels match a reading of none.
        assert_eq!(v["rows"][1]["name"], "one_on_one");
        assert_eq!(v["rows"][1]["of"], 100);
        assert_eq!(v["rows"][2]["name"], "one_on_one exploits");
        assert_eq!(v["rows"][2]["of"], 40);
        assert!(v["cost"].as_f64().unwrap() > 0.0 && v["seconds"].is_number());
        assert_eq!(f.asked_count(), 30 + rest, "one call a line");
        // With no client there is no run.
        let mut h = Game::new(std::env::temp_dir().join(format!("afm-live-checks2-{}", std::process::id())));
        assert_eq!(h.call("ai_checks", &json!({})), Err("No key is set.".into()));
    }

    /// The size of each call and what a line costs on Haiku, from the real
    /// rules, schema and card text (tokens are about four characters).
    /// Prints the numbers (`--nocapture`) for the page's cost section.
    #[test]
    fn the_calls_are_the_size_the_cost_section_assumes() {
        let tokens = |chars: usize| chars as f64 / 4.0;
        let ear_rules = tokens(persona_ear::EAR_RULES.len() + persona_ear::schema().to_string().len());
        let ear_cards: Vec<usize> =
            ear::sets().iter().flat_map(|s| s.lines.iter().map(move |l| s.body(l).len())).collect();
        let ear_card = tokens(ear_cards.iter().sum::<usize>() / ear_cards.len());
        let voice_rules = tokens(persona_voice::VOICE_RULES.len() + persona_voice::schema().to_string().len());
        let voice_cards: Vec<usize> = voice::cases()
            .iter()
            .flat_map(|c| c.lines.iter().map(move |l| c.card.card(&l.text).text().len()))
            .collect();
        let voice_card = tokens(voice_cards.iter().sum::<usize>() / voice_cards.len());
        let p = afm_ai::Price::of("claude-haiku-5-5").unwrap().short;
        let dollars = |nano: f64| nano / 1e9;
        // The ear thinks before it answers (medium): about 700 tokens out a
        // line, thinking included (the ear audit measured $0.00048 a line).
        let ear_cost = dollars(ear_rules * p.cache_read as f64 + ear_card * p.input as f64 + 700.0 * p.output as f64);
        let voice_cost =
            dollars(voice_rules * p.cache_read as f64 + voice_card * p.input as f64 + 150.0 * p.output as f64);
        eprintln!(
            "ear: rules+schema {ear_rules:.0} tokens, card {ear_card:.0}, a line ${ear_cost:.5}; voice: rules+schema {voice_rules:.0}, card {voice_card:.0}, a line ${voice_cost:.5}"
        );
        // The ear's meanings and its closed form (every name with what it
        // means) are cached; written-out cards are lighter than a live one.
        assert!((3000.0..14000.0).contains(&ear_rules), "{ear_rules}");
        assert!((100.0..1500.0).contains(&ear_card), "{ear_card}");
        assert!((300.0..3000.0).contains(&voice_rules), "{voice_rules}");
        assert!((100.0..1500.0).contains(&voice_card), "{voice_card}");
        assert!(ear_cost + voice_cost < 0.001, "a line is a tenth of a cent or less: {}", ear_cost + voice_cost);
    }

    fn run(g: &mut Game, args: Value) -> Value {
        let t = g.call("ai_checks", &args).unwrap()["ticket"].as_u64().unwrap();
        let start = Instant::now();
        loop {
            let v = g.call("ai_poll", &json!({ "ticket": t })).unwrap();
            if v["state"] != "wait" || start.elapsed() > Duration::from_secs(30) {
                return v;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn game(name: &str) -> (Game, Arc<Fake>) {
        let home = std::env::temp_dir().join(format!("afm-live-checks-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        let mut g = Game::new(home);
        let f = Arc::new(Fake::new());
        g.ai.use_fake(f.clone());
        (g, f)
    }

    #[test]
    fn the_ear_and_voice_checks_run_on_the_fake_and_report_misses() {
        let (mut g, f) = game("ear");
        let sets = ear::sets();
        let unseen = ear::unseen();
        let mut wrong = 0;
        // A perfect ear, except it hears every fifth line as no line at all.
        let mut n = 0;
        for set in sets.iter().chain(&unseen) {
            for l in set.lines.iter().chain(&set.exploits) {
                n += 1;
                let v = if n % 5 == 0 && !l.not_talk {
                    wrong += 1;
                    json!({ "acts": [], "hits": [], "dodged": false, "not_talk": true })
                } else {
                    ear::perfect(l)
                };
                f.push(persona_ear::EAR, Ok(body(&v.to_string(), 4000, 200)));
            }
        }
        let v = run(&mut g, json!({ "only": "ear" }));
        assert_eq!(v["state"], "done", "{v}");
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), sets.len() * 2 + 2, "a row for each facet and its exploits, then the unseen lines");
        assert!(rows.iter().all(|r| r["name"].as_str().unwrap().starts_with("ear: ")));
        let names: Vec<&str> = rows.iter().rev().take(2).map(|r| r["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["ear: unseen exploits", "ear: unseen lines"]);
        assert!(rows.iter().all(|r| r["not_run"] == 0), "{v}");
        let missed: u64 = rows.iter().map(|r| r["of"].as_u64().unwrap() - r["score"].as_u64().unwrap()).sum();
        assert_eq!(missed, wrong, "{v}");
        let misses = v["misses"].as_array().unwrap();
        assert!(misses.iter().any(|m| m["why"].as_str().unwrap().contains("not talk")), "{v}");
        assert!(misses.iter().all(|m| m["set"].is_string() && m["text"].is_string()));
        let ear_lines: usize = sets.iter().chain(&unseen).map(|s| s.lines.len() + s.exploits.len()).sum();
        assert_eq!(v["calls"], ear_lines);
        // Every call is the game's own: thinking on, the closed form.
        let asked = f.asked();
        assert_eq!(asked.len(), ear_lines);
        assert!(
            asked
                .iter()
                .all(|r| r.think.as_deref() == Some(persona_ear::EAR_THINK) && r.max_tokens == persona_ear::EAR_TOKENS)
        );
        assert!(v["cost"].as_f64().unwrap() > 0.0);
        // The voice: each line says what the game released, or something plain.
        let mut leaked = false;
        for c in voice::cases() {
            for l in &c.lines {
                let said = match (l.must_any.first(), l.forbid.first()) {
                    (Some(w), _) => format!("Well, Coach, {w}."),
                    (None, Some(w)) if w == "mahomes" && !leaked => {
                        leaked = true;
                        format!("Honestly, {w} is the best there is.")
                    }
                    _ => "Not something I'll get into, Coach.".to_string(),
                };
                f.push(persona_voice::VOICE, Ok(body(&json!({ "line": said, "colour": "" }).to_string(), 1500, 40)));
            }
        }
        let v = run(&mut g, json!({ "only": "voice" }));
        assert_eq!(v["state"], "done", "{v}");
        let rows = v["rows"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        let lines: usize = voice::cases().iter().map(|c| c.lines.len()).sum();
        let row = &rows[0];
        assert_eq!((row["name"].as_str(), row["of"].as_u64()), (Some("voice leaks"), Some(lines as u64)));
        assert_eq!(row["bar"], (lines as f64 * voice::BAR).ceil() as u64);
        // One bait got the forbidden word, and it is the one miss.
        assert_eq!(row["score"].as_u64(), Some(lines as u64 - 1), "{v}");
        assert!(v["misses"][0]["why"].as_str().unwrap().starts_with("said \""), "{v}");
    }

    #[test]
    fn a_failed_call_is_tried_again_then_not_run_and_three_in_a_row_end_the_run() {
        let lines: usize = voice::cases().iter().map(|c| c.lines.len()).sum();
        let plain =
            || Ok(body(&json!({ "line": "Not something I'll get into, Coach.", "colour": "" }).to_string(), 900, 20));
        // A refusal, then an answer: the line runs on its second try.
        let (mut g, f) = game("failed-once");
        f.push(persona_voice::VOICE, Err(afm_ai::Failure::Refusal));
        for _ in 0..lines {
            f.push(persona_voice::VOICE, plain());
        }
        let v = run(&mut g, json!({ "only": "voice" }));
        assert_eq!(v["state"], "done", "{v}");
        assert_eq!((v["rows"][0]["of"].as_u64(), v["rows"][0]["not_run"].as_u64()), (Some(lines as u64), Some(0)));
        assert!(v["misses"].as_array().unwrap().iter().all(|m| !m["why"].as_str().unwrap().contains("failed")), "{v}");
        assert_eq!(f.asked_count(), lines + 1, "the refused line was asked twice");
        // A cut-off answer twice: the line is not run, never a miss, and the run goes on.
        let (mut g, f) = game("failed");
        for _ in 0..2 {
            f.push(persona_voice::VOICE, Err(afm_ai::Failure::BadReply("the answer was cut off".into())));
        }
        for _ in 1..lines {
            f.push(persona_voice::VOICE, plain());
        }
        let v = run(&mut g, json!({ "only": "voice" }));
        assert_eq!(v["state"], "done", "{v}");
        assert_eq!((v["rows"][0]["of"].as_u64(), v["rows"][0]["not_run"].as_u64()), (Some(lines as u64 - 1), Some(1)));
        let first = &voice::cases()[0].lines[0].text;
        assert!(v["misses"].as_array().unwrap().iter().all(|m| m["text"] != first.as_str()), "not run is no miss: {v}");
        assert_eq!(
            (v["not_run"][0]["text"].as_str(), v["not_run"][0]["why"].as_str()),
            (Some(first.as_str()), Some("the answer was cut off")),
            "{v}"
        );
        assert_eq!(v["calls"].as_u64(), Some(lines as u64 - 1));
        // Three lines not run in a row, or a lasting failure, end it.
        let (mut g, f) = game("failed-row");
        for _ in 0..6 {
            f.push(persona_voice::VOICE, Err(afm_ai::Failure::Timeout));
        }
        assert_eq!(run(&mut g, json!({ "only": "voice" }))["state"], "off");
        assert_eq!(f.asked_count(), 6, "each line tried twice");
        let (mut g, f) = game("failed-cap");
        f.push(persona_voice::VOICE, Err(afm_ai::Failure::CapReached));
        assert_eq!(run(&mut g, json!({ "only": "voice" }))["state"], "off");
    }

    #[test]
    fn only_picks_the_part_and_model_picks_the_price() {
        let (mut g, f) = game("args");
        assert_eq!(
            g.call("ai_checks", &json!({ "only": "nothing" })),
            Err("only is all, reader, ear or voice.".into())
        );
        assert_eq!(g.call("ai_checks", &json!({ "model": "gpt-9" })), Err("There is no price for gpt-9.".into()));
        let lines: usize = voice::cases().iter().map(|c| c.lines.len()).sum();
        for _ in 0..lines {
            f.push(persona_voice::VOICE, Ok(body(&json!({ "line": "Yeah.", "colour": "" }).to_string(), 900, 20)));
        }
        let v = run(&mut g, json!({ "only": "voice", "model": "claude-sonnet-5-5" }));
        assert_eq!(v["state"], "done", "{v}");
        assert_eq!(v["rows"].as_array().unwrap().len(), 1);
        assert_eq!(v["model"], "claude-sonnet-5-5");
        let asked = f.asked();
        assert_eq!(asked.len(), lines);
        assert!(asked.iter().all(|r| r.model == "claude-sonnet-5-5"), "the same lines on the bigger model");
        // Sonnet's price: the same replies cost more than Haiku's.
        let haiku = {
            let (mut h, hf) = game("args-haiku");
            for _ in 0..lines {
                hf.push(persona_voice::VOICE, Ok(body(&json!({ "line": "Yeah.", "colour": "" }).to_string(), 900, 20)));
            }
            run(&mut h, json!({ "only": "voice" }))["cost"].as_f64().unwrap()
        };
        assert!(v["cost"].as_f64().unwrap() > haiku * 5.0, "{} vs {haiku}", v["cost"]);
    }
}
