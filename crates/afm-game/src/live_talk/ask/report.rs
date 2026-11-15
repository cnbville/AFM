//! The scouting report in prose (live talks lane 4, LT14): a button on a
//! player's file, "Write the report". The game gathers what your staff has
//! read (`scout_file`: where he is now, strengths and weaknesses, character
//! reads, habits, medical, testing, academics, who he compares to and what the
//! room says), in sections. The writer turns them into a short report under
//! those headings. The filter checks every paragraph against the file: a
//! number that is not in it, a banned word or a heading it never gave drops
//! the report, and the fallback is today's set lines, the file's own words
//! under the same headings.

use afm_ai::Request;
use serde_json::{Value, json};

use super::analyst::{none, start};
use super::scout::sheet;
use crate::live_talk::filter;
use crate::live_talk::tickets::{Next, Step};
use crate::{Game, Reply};

pub const WRITER: &str = "report_writer";

/// The report's longest answer, in tokens. Proposed, not confirmed.
const REPORT_TOKENS: u32 = 800;
/// The most lines a report may run to (headings and paragraphs).
const MAX_LINES: usize = 30;

const RULES: &str = "You write a scouting report on one football player for a head coach, for a football management game. The game has already decided what the staff knows: the card gives the file in sections. Your words must say what the file says. You never decide anything yourself.

Rules:
- Write one short paragraph of plain prose (one to three sentences) for each section of the card, under its heading. A heading is a line that starts with \"## \" and holds the section's name exactly as the card gives it. Skip a section only if the card has none.
- Write only prose: no lists, no bullet points, no bold, no other marks.
- Keep ranges as ranges and keep the file's own words for how sure the staff is. Never make a read sound surer than the card does.
- Add no fact, name, number, date or result that is not in the card.
- Never mention ratings, overalls, potential, ceilings, attributes or anything about the game itself.";

/// A report, or why it can't be used: every heading is one the card gave and
/// every paragraph passes the filter against the card.
fn checked(text: &str, card: &str, headings: &[&str]) -> Option<String> {
    let mut out = Vec::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(h) = line.strip_prefix("## ") {
            let h = h.trim();
            if !headings.contains(&h) {
                return None;
            }
            out.push(format!("## {h}"));
        } else {
            out.push(filter::check(line, card).ok()?);
        }
    }
    (!out.is_empty() && out.len() <= MAX_LINES && out[0].starts_with("## ")).then(|| out.join("\n"))
}

/// `ai_ask {kind: "report", player}`.
pub fn ask(g: &mut Game, args: &Value) -> Reply {
    let id = args.get("player").and_then(Value::as_u64).ok_or("Missing player.")? as u32;
    let w = g.world.as_ref().ok_or("No world is loaded.")?;
    let file = crate::lane_scouting::scout_file(w, id)?;
    if file.get("seen").and_then(Value::as_bool) != Some(true) {
        return Err("Your staff has no file on him yet.".into());
    }
    let sheet = sheet(&file);
    if sheet.sections.is_empty() {
        return Err("There is nothing in his file to write from yet.".into());
    }
    let mut card = format!("The player: {}.\n", sheet.who);
    let mut set = Vec::new();
    for (head, lines) in &sheet.sections {
        card.push_str(&format!("\nSection: {head}\n"));
        for l in lines {
            card.push_str(&format!("- {l}\n"));
        }
        set.push(format!("## {head}\n{}", lines.join("\n")));
    }
    let fallback = set.join("\n");
    let heads: Vec<String> = sheet.sections.iter().map(|s| s.0.to_string()).collect();
    let req = Request::new(WRITER, RULES, &card).max_tokens(REPORT_TOKENS);
    let next: Next = Box::new(move |_g, r| {
        let h: Vec<&str> = heads.iter().map(String::as_str).collect();
        let text = r.ok().and_then(|reply| checked(&reply.text, &card, &h)).unwrap_or(fallback);
        if text.is_empty() {
            return none("Nothing was written.");
        }
        Step::Done(json!({ "state": "done", "text": text }))
    });
    start(g, req, next)
}

/// Adds this ask's entry to a reply's `ask` list (`live_talk::after`, only
/// with a usable key; `super::push` adds one): the button, on a file your
/// staff has read.
pub fn flag(_g: &Game, cmd: &str, v: &mut Value) {
    if cmd != "scout_file" || v.get("seen").and_then(Value::as_bool) != Some(true) {
        return;
    }
    let Some(id) = v.get("id").and_then(Value::as_u64) else { return };
    super::push(
        v,
        json!({ "cmd": "ai_ask", "args": { "kind": "report", "player": id }, "button": "Write the report" }),
    );
}

#[cfg(test)]
mod tests {
    use super::checked;

    const CARD: &str =
        "The player: Sam Hill, WR, Junior, age 20.\n\nSection: Where he is now\n- Where he is now: 58 to 66\n";

    #[test]
    fn a_report_keeps_its_headings_and_passes_the_filter() {
        let ok = "## Where he is now\nHe looks like somewhere between 58 and 66 today.";
        assert_eq!(checked(ok, CARD, &["Where he is now"]).as_deref(), Some(ok));
        // A heading the card never gave.
        assert!(checked("## His ceiling\nA star.", CARD, &["Where he is now"]).is_none());
        // A number the card doesn't hold.
        assert!(checked("## Where he is now\nHe is a 72 now.", CARD, &["Where he is now"]).is_none());
        // A banned word, markup, nothing at all, prose before the first heading.
        assert!(checked("## Where he is now\nHis potential is high.", CARD, &["Where he is now"]).is_none());
        assert!(checked("## Where he is now\n**Good.**", CARD, &["Where he is now"]).is_none());
        assert!(checked("", CARD, &["Where he is now"]).is_none());
        assert!(checked("He is fine.\n## Where he is now\nOk.", CARD, &["Where he is now"]).is_none());
    }
}
