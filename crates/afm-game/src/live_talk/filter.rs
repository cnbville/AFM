//! The reply filter (live talks, the core): the writer's words reach the
//! screen only when they pass. A reply that names ratings, overall,
//! potential, a ceiling or attributes, or carries a number its card doesn't
//! hold, is dropped, and the line the game already picked (the line bank's,
//! or the scene's own) stays. A tricked writer can only change words, never
//! a number, and this keeps even the words honest about what he knows.

/// Words a reply may never use: the game's insides. Whole words, any case.
pub const BANNED: [&str; 16] = [
    "rating",
    "ratings",
    "rated",
    "overall",
    "overalls",
    "ovr",
    "potential",
    "ceiling",
    "ceilings",
    "attribute",
    "attributes",
    "stat-line",
    "engine",
    "simulation",
    "algorithm",
    "prompt",
];

/// The longest reply kept, in characters (three short sentences).
pub const MAX_CHARS: usize = 600;

/// Why a reply was dropped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dropped {
    Empty,
    TooLong,
    /// It carries markup or a tag.
    Markup,
    Banned(String),
    /// A number the card doesn't hold.
    Number(String),
}

/// The reply as the screen may show it, or why it was dropped. `card` is
/// everything the writer was given (its card and your line): a number in
/// the reply must be one of the card's.
pub fn check(reply: &str, card: &str) -> Result<String, Dropped> {
    let t = reply.trim().trim_matches(|c| c == '"' || c == '\u{201c}' || c == '\u{201d}').trim();
    if t.is_empty() {
        return Err(Dropped::Empty);
    }
    if t.chars().count() > MAX_CHARS {
        return Err(Dropped::TooLong);
    }
    if t.contains(['<', '>', '{', '}', '[', ']', '*', '#']) {
        return Err(Dropped::Markup);
    }
    for word in words(t) {
        if BANNED.contains(&word.as_str()) {
            return Err(Dropped::Banned(word));
        }
    }
    let allowed = numbers(card);
    for n in numbers(t) {
        if !allowed.contains(&n) {
            return Err(Dropped::Number(n.to_string()));
        }
    }
    Ok(t.to_string())
}

/// The words of a text, lower case: letters, digits, `-` and `'` kept.
fn words(t: &str) -> Vec<String> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '\''))
        .filter(|w| !w.is_empty())
        .map(|w| w.trim_matches(|c| c == '\'' || c == '-').to_lowercase())
        .collect()
}

/// Number words a reply might use instead of digits. "one" is left out:
/// it is mostly not a number ("the one who", "one more thing").
const NUMBER_WORDS: [(&str, u64); 30] = [
    ("two", 2),
    ("three", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
    ("hundred", 100),
    ("thousand", 1_000),
    ("million", 1_000_000),
    ("dozen", 12),
];

/// Every number a text holds: digit runs (commas and points inside a
/// number are part of it: "1,200", "4.5"), and number words, with
/// "twenty-two" read as 22.
pub fn numbers(t: &str) -> Vec<u64> {
    let mut out = Vec::new();
    let chars: Vec<char> = t.chars().collect();
    let mut k = 0;
    while k < chars.len() {
        if chars[k].is_ascii_digit() {
            let mut s = String::new();
            while k < chars.len()
                && (chars[k].is_ascii_digit()
                    || (matches!(chars[k], ',' | '.') && k + 1 < chars.len() && chars[k + 1].is_ascii_digit()))
            {
                if chars[k] != ',' {
                    s.push(chars[k]);
                }
                k += 1;
            }
            // "4.5" counts as 45 tenths and as its whole part, so either
            // spelling in the card allows it.
            let whole = s.split('.').next().unwrap_or("");
            if let Ok(n) = whole.parse::<u64>() {
                out.push(n);
            }
            if s.contains('.')
                && let Ok(n) = s.replace('.', "").parse::<u64>()
            {
                out.push(n);
            }
        } else {
            k += 1;
        }
    }
    for w in words(t) {
        let parts: Vec<u64> =
            w.split('-').filter_map(|p| NUMBER_WORDS.iter().find(|(name, _)| *name == p).map(|x| x.1)).collect();
        if !parts.is_empty() {
            out.push(parts.iter().sum());
            out.extend(parts.iter().copied());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CARD: &str = "Him: Marcus Hill, QB, redshirt senior. 30 career starts. His line: \"I've started thirty \
                        games here.\" <coach_line>You're still my guy on third downs, and you had 212 yards.</coach_line>";

    /// 200 replies: 100 a player might say, 100 that must be dropped.
    #[test]
    fn the_filter_on_200_replies() {
        let good = [
            "Third downs? I've started thirty games here, Coach.",
            "I hear you. I don't agree with you.",
            "That's not what I was told before.",
            "That hurts, Coach. I'm not going to lie.",
            "212 yards and I'm on the bench?",
            "Thirty starts, Coach. Thirty.",
            "Fine. I'll be ready when you need me.",
            "\"I get it. I don't like it, but I get it.\"",
            "You know I'll run through a wall for this team.",
            "Then tell me what I have to do to get it back.",
        ];
        let bad = [
            "My overall is better than his.",
            "I have more potential than the freshman.",
            "Coach, my ceiling is higher.",
            "Look at my ratings.",
            "My attributes say otherwise.",
            "I've started 31 games here.",
            "I threw for 400 yards last week.",
            "Twenty-five starts and this is what I get?",
            "I'm a five-star, Coach.",
            "<b>I'm angry.</b>",
        ];
        let mut kept = 0;
        let mut dropped = 0;
        for k in 0..10 {
            for g in good {
                let r = if k % 2 == 0 { g.to_string() } else { format!("  {g}  ") };
                assert!(check(&r, CARD).is_ok(), "{r:?}: {:?}", check(&r, CARD));
                kept += 1;
            }
            for b in bad {
                let r = if k % 2 == 0 { b.to_string() } else { b.to_uppercase() };
                assert!(check(&r, CARD).is_err(), "{r:?} passed");
                dropped += 1;
            }
        }
        assert_eq!((kept, dropped), (100, 100));
    }

    #[test]
    fn why_a_reply_is_dropped() {
        assert_eq!(check("   ", CARD), Err(Dropped::Empty));
        assert_eq!(check(&"a".repeat(MAX_CHARS + 1), CARD), Err(Dropped::TooLong));
        assert_eq!(check("He has the overall edge.", CARD), Err(Dropped::Banned("overall".into())));
        assert_eq!(check("I had 3 picks.", CARD), Err(Dropped::Number("3".into())));
        assert_eq!(check("Give me two weeks.", CARD), Err(Dropped::Number("2".into())));
        assert!(check("It's 1,200 miles from home.", "1,200 miles").is_ok());
        assert!(check("He ran a 4.5.", "a 4.5 forty").is_ok());
        assert!(check("Ninety-nine problems.", "99").is_ok());
        // "one" and the ordinals are words, not counts.
        assert!(check("One more thing: I'm the one for third downs.", CARD).is_ok());
    }
}
