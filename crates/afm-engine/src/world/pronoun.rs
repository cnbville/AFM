//! He or she for a coach in the world's words. Your own coach is what you
//! picked on the setup page (`seat::You::she`, minor bug 178's follow-up,
//! Cain's yes 2026-11-02); every coach the world makes is a man. Cast
//! people (bosses, fans, families) carry their own (`folk::cast::Cast::she`).

use super::World;

/// One person's third-person words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pro {
    pub she: bool,
}

impl Pro {
    pub const HE: Pro = Pro { she: false };
    pub const SHE: Pro = Pro { she: true };
    pub fn he(self) -> &'static str {
        if self.she { "she" } else { "he" }
    }
    pub fn him(self) -> &'static str {
        if self.she { "her" } else { "him" }
    }
    pub fn his(self) -> &'static str {
        if self.she { "her" } else { "his" }
    }
    /// The possessive standing alone: "the job is his".
    pub fn hers(self) -> &'static str {
        if self.she { "hers" } else { "his" }
    }
    pub fn himself(self) -> &'static str {
        if self.she { "herself" } else { "himself" }
    }
    /// "He" at the start of a sentence.
    pub fn he_cap(self) -> &'static str {
        if self.she { "She" } else { "He" }
    }
    pub fn his_cap(self) -> &'static str {
        if self.she { "Her" } else { "His" }
    }
    /// "He's" / "She's".
    pub fn hes(self) -> &'static str {
        if self.she { "She's" } else { "He's" }
    }
    pub fn man(self) -> &'static str {
        if self.she { "woman" } else { "man" }
    }
    /// What a player calls a coach to his face: "Yes sir" / "Yes ma'am".
    pub fn sir(self) -> &'static str {
        if self.she { "ma'am" } else { "sir" }
    }
    /// A stored phrase written for a man ("not retained when his contract
    /// ran out") in this person's words. The stored phrase stays the key
    /// the world and its book match on; only what you read changes.
    pub fn say(self, text: &str) -> String {
        if !self.she {
            return text.to_string();
        }
        swap(text, |w| match w {
            "he" => Some("she"),
            "He" => Some("She"),
            "him" | "his" => Some("her"),
            "His" => Some("Her"),
            "himself" => Some("herself"),
            _ => None,
        })
    }
}

/// Your coach's words.
pub fn you(w: &World) -> Pro {
    Pro { she: w.career.as_ref().is_some_and(|k| k.you.she) }
}

/// A coach's words: yours as you set them, every other coach's he.
pub fn coach(w: &World, c: usize) -> Pro {
    Pro { she: w.career.as_ref().is_some_and(|k| k.coach == c && k.you.she) }
}

/// A stored phrase about your coach said to you: "fired after your
/// prove-it year".
pub fn to_you(text: &str) -> String {
    swap(text, |w| match w {
        "he" | "him" => Some("you"),
        "He" => Some("You"),
        "his" => Some("your"),
        "His" => Some("Your"),
        "himself" => Some("yourself"),
        _ => None,
    })
}

/// Swaps whole words.
fn swap(text: &str, f: impl Fn(&str) -> Option<&'static str>) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let mut word = String::new();
    for ch in text.chars().chain(std::iter::once('\0')) {
        if ch.is_alphabetic() {
            word.push(ch);
            continue;
        }
        out.push_str(f(&word).unwrap_or(&word));
        word.clear();
        if ch != '\0' {
            out.push(ch);
        }
    }
    out
}
