//! Small helpers for the notes and news the engine writes.

/// The article a word takes when read aloud: "an" before a vowel sound.
/// Abbreviations are read letter by letter, so "an EDGE", "an OT", "an LB",
/// "an S", "an RB", "an LS" and "an FCS", but "a QB" and "a UCLA". Numbers
/// are read as words: "an 8", "an 11", "an 18-year-old", "an 80" and "an
/// 11,000", but "a 1", "a 10", "a 48" and "a 180".
pub fn a_or_an(word: &str) -> &'static str {
    let w = word.trim_start();
    let Some(first) = w.chars().next() else { return "a" };
    let vowel = if first.is_ascii_digit() {
        // The number's first spoken group: the digits before the first
        // thousands comma, or the leading 1-3 digits of a plain number.
        let digits: String = w.chars().take_while(|c| c.is_ascii_digit() || *c == ',').collect();
        let plain: String = digits.chars().filter(|c| *c != ',').collect();
        let lead = if digits.contains(',') {
            digits.split(',').next().unwrap_or("").to_string()
        } else {
            let k = match plain.len() % 3 {
                0 => 3,
                k => k,
            };
            plain[..k.min(plain.len())].to_string()
        };
        lead.starts_with('8') || lead == "11" || lead == "18"
    } else {
        let token: String = w.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
        let lower = token.to_ascii_lowercase();
        let initials = token.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit());
        if initials && !matches!(token.as_str(), "MAC" | "MEAC" | "SWAC" | "JUCO") {
            // Read letter by letter: A, E, F, H, I, L, M, N, O, R, S and X
            // start with a vowel sound; U is read "you".
            "AEFHILMNORSX".contains(first)
        } else if lower.starts_with('u') {
            // "an unranked", "an upset"; but "a Utah", "a university".
            (lower.starts_with("un") && !lower.starts_with("uni")) || lower.starts_with("up")
        } else {
            "aeio".contains(lower.chars().next().unwrap_or('x'))
                || lower.starts_with("hon")
                || lower.starts_with("hour")
        }
    };
    if vowel { "an" } else { "a" }
}

/// The word with its article: "an EDGE", "a QB", "an 18".
pub fn with_article(word: &str) -> String {
    format!("{} {word}", a_or_an(word))
}
