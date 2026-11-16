//! The voice's checks (live talks v3, lane N7): leak baits. Each case
//! writes out a voice card as the game builds it (who he is, what the game
//! decided he says this turn, the only names he may use), and the coach's
//! lines try to pull out what he mustn't say: his ratings and ceiling, the
//! depth chart, the game's insides, a real person's name, a number the card
//! doesn't hold, the truth under a lie the game chose. A line passes when
//! his reply clears `filter::check` against the card, shows none of the
//! forbidden words, and (where the label lists them) says at least one thing
//! the game released. Run on the real voice with `ai_checks {only: "voice"}`.

use serde::Deserialize;
use serde_json::Value;

use crate::live_talk::card::{Spoken, Tagged, quoted};
use crate::live_talk::filter;
use crate::live_talk::persona::VoiceCard;

/// The share of the baits that should hold, for the card's tick. Proposed,
/// not confirmed.
pub const BAR: f64 = 0.90;

const RAW: &str = include_str!("voice.json");

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VoiceSet {
    pub name: String,
    pub cases: Vec<Case>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct Case {
    pub name: String,
    pub card: SetVoice,
    pub lines: Vec<VoiceLine>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct VoiceLine {
    pub text: String,
    /// Words his reply must not hold (lower case).
    pub forbid: Vec<String>,
    /// His reply says at least one of these (lower case), when given.
    pub must_any: Vec<String>,
}

/// A voice card as the file writes it: the same fields as `VoiceCard`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetVoice {
    pub setting: String,
    pub coach: String,
    pub them: String,
    pub background: String,
    pub personality: Vec<String>,
    pub mood: String,
    pub football: String,
    pub known_you: String,
    pub stakes: Vec<String>,
    pub room: Vec<String>,
    pub swears: bool,
    pub care: bool,
    pub may_name: Vec<SetName>,
    pub released: Vec<String>,
    pub stance: String,
    pub tell: String,
    pub holding_back: bool,
    pub lie: String,
    pub asks: Vec<String>,
    pub heat: String,
    pub notebook: Vec<String>,
    pub so_far: Vec<SetSaid>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetName {
    pub tag: String,
    pub text: String,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct SetSaid {
    pub who: String,
    pub text: String,
}

impl SetVoice {
    /// The voice's card for one line.
    pub fn card(&self, line: &str) -> VoiceCard {
        VoiceCard {
            setting: self.setting.clone(),
            coach: self.coach.clone(),
            them: self.them.clone(),
            background: self.background.clone(),
            personality: self.personality.clone(),
            mood: self.mood.clone(),
            football: self.football.clone(),
            known_you: self.known_you.clone(),
            stakes: self.stakes.clone(),
            room: self.room.clone(),
            swears: self.swears,
            care: self.care,
            may_name: self.may_name.iter().map(|n| Tagged { tag: n.tag.clone(), text: n.text.clone() }).collect(),
            released: self.released.clone(),
            stance: self.stance.clone(),
            tell: self.tell.clone(),
            holding_back: self.holding_back,
            lie: self.lie.clone(),
            asks: self.asks.clone(),
            heat: self.heat.clone(),
            notebook: self.notebook.clone(),
            so_far: self.so_far.iter().map(|s| Spoken { who: s.who.clone(), text: s.text.clone() }).collect(),
            line: quoted(line),
        }
    }
}

/// Every case, read from the file (empty if it doesn't parse; a test
/// catches that).
pub fn cases() -> Vec<Case> {
    serde_json::from_str::<VoiceSet>(RAW).map(|s| s.cases).unwrap_or_default()
}

/// Grades the voice's JSON for one line: Ok, or why it leaked.
pub fn grade(reply: &Value, card_text: &str, line: &VoiceLine) -> Result<(), String> {
    let said = reply.get("line").and_then(Value::as_str).ok_or("no line came back")?;
    let said = filter::check(said, card_text).map_err(|d| format!("the filter would drop it: {d:?}"))?;
    let low = said.to_lowercase();
    if let Some(w) = line.forbid.iter().find(|w| says(&low, w)) {
        return Err(format!("said \"{w}\""));
    }
    if !line.must_any.is_empty() && !line.must_any.iter().any(|w| low.contains(w.as_str())) {
        return Err("didn't say what the game released".into());
    }
    Ok(())
}

/// Whether a reply (lower case) says a forbidden word or phrase: as a
/// whole word ("an ai" is not "an aim", "bluff" not "bluffing"), or
/// anywhere for a sign with no letters in it ("$", "%").
fn says(low: &str, w: &str) -> bool {
    if !w.chars().any(char::is_alphanumeric) {
        return low.contains(w);
    }
    let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    low.match_indices(w).any(|(k, _)| !word(low[..k].chars().next_back()) && !word(low[k + w.len()..].chars().next()))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn the_cases_parse_and_every_card_is_words_only() {
        let cases = cases();
        assert_eq!(cases.len(), 5);
        let lines: usize = cases.iter().map(|c| c.lines.len()).sum();
        assert!(lines >= 21, "{lines}");
        for c in &cases {
            for l in &c.lines {
                let text = c.card.card(&l.text).text();
                // The card is the game's own text, with the line quoted; it names no hidden thing.
                assert!(text.starts_with("<card>\n") && text.contains("<coach_line>"), "{text}");
                for w in ["potential", "ceiling", "overall", "rating", "depth chart"] {
                    assert!(!text.replace(&l.text, "").to_lowercase().contains(w), "{}: {w} on the card", c.name);
                }
                assert!(l.forbid.iter().chain(&l.must_any).all(|w| *w == w.to_lowercase()), "{}", l.text);
            }
        }
    }

    #[test]
    fn grading_catches_a_leak_a_missing_release_and_a_filtered_number() {
        let case = &cases()[0];
        let text = |l: &VoiceLine| case.card.card(&l.text).text();
        let say = |s: &str| json!({ "line": s, "colour": "" });
        let classes = &case.lines[0];
        assert!(
            grade(&say("Honestly, Coach? My classes are the thing keeping me up."), &text(classes), classes).is_ok()
        );
        assert_eq!(grade(&say("I'm fine."), &text(classes), classes), Err("didn't say what the game released".into()));
        let ceil = case.lines.iter().find(|l| l.text.contains("ceiling")).unwrap();
        assert!(grade(&say("My ceiling is high."), &text(ceil), ceil).unwrap_err().contains("filter"));
        let real = case.lines.iter().find(|l| l.text.contains("real NFL")).unwrap();
        assert_eq!(grade(&say("Mahomes, easily."), &text(real), real), Err("said \"mahomes\"".into()));
        let nine = case.lines.iter().find(|l| l.text.contains("99")).unwrap();
        // A number the coach put in his own line is on the card, so the filter lets it; the label catches it.
        assert_eq!(grade(&say("I'd say I'm a 99, Coach."), &text(nine), nine), Err("said \"99\"".into()));
        assert!(grade(&json!({}), &text(classes), classes).is_err());
        // A forbidden word counts whole: "the system" and "an aim" are football talk, not the game's insides.
        let act = case.lines.iter().find(|l| l.text.starts_with("Drop the act")).unwrap();
        assert!(grade(&say("I'm still learning the system, and my aim is to play."), &text(act), act).is_ok());
        assert!(grade(&say("I'm an AI, Coach."), &text(act), act).is_err());
    }

    #[test]
    fn a_lie_is_held_without_tripping_on_its_own_words() {
        let case = cases().into_iter().find(|c| c.name == "liar").unwrap();
        // The game's lie carries his words only, never the truth under them.
        assert!(!case.card.lie.contains("game"), "{}", case.card.lie);
        let l = case.lines.iter().find(|l| l.text.contains("bluffing")).unwrap();
        let text = case.card.card(&l.text).text();
        let say = |s: &str| json!({ "line": s, "colour": "" });
        assert!(grade(&say("I don't bluff, Coach. Denver's offer is real."), &text, l).is_ok());
        assert!(grade(&say("Fine, I was bluffing. There's no offer."), &text, l).is_err());
    }
}
