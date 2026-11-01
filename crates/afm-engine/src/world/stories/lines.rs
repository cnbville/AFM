//! The stories' line banks (`data/lines/stories/*.txt`, format in
//! `FORMAT.md` beside them): templates by key, some only for a span of
//! seasons, filled from named values. A template that names a value the
//! caller didn't give is skipped, so a key always reads whole.
//!
//! Repeats are kept down per key: the last lines used under a key rest
//! before they can come back (people topic 5's "a pattern rests for about a
//! season" applied to the whole world's text).

use std::collections::HashMap;
use std::sync::LazyLock;

use crate::rng::Rng;

const NICKNAMES: &str = include_str!("../../../../../data/lines/stories/nicknames.txt");
const STORIES: &str = include_str!("../../../../../data/lines/stories/stories.txt");
const STORYLINES: &str = include_str!("../../../../../data/lines/stories/storylines.txt");

/// One template and the seasons it may be used in.
struct Line {
    text: &'static str,
    from: i32,
    to: i32,
}

pub(crate) struct Bank {
    keys: HashMap<&'static str, Vec<Line>>,
    /// Real nicknames and named plays the game never makes (lowercase).
    blocked: Vec<String>,
}

fn parse(src: &'static str, bank: &mut Bank) {
    let mut key: Option<(&'static str, i32, i32)> = None;
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(head) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            let (name, span) = head.split_once('@').unwrap_or((head, ""));
            let (from, to) = match span.split_once('-') {
                Some((a, b)) => (a.trim().parse().unwrap_or(i32::MIN), b.trim().parse().unwrap_or(i32::MAX)),
                None => (i32::MIN, i32::MAX),
            };
            key = Some((name.trim(), from, to));
            continue;
        }
        let Some((name, from, to)) = key else { continue };
        if name == "blocklist" {
            bank.blocked.push(line.to_lowercase());
            continue;
        }
        bank.keys.entry(name).or_default().push(Line { text: line, from, to });
    }
}

pub(crate) static BANK: LazyLock<Bank> = LazyLock::new(|| {
    let mut b = Bank { keys: HashMap::new(), blocked: Vec::new() };
    parse(NICKNAMES, &mut b);
    parse(STORIES, &mut b);
    parse(STORYLINES, &mut b);
    b
});

/// Named values for a template.
#[derive(Default, Clone)]
pub(crate) struct Fill {
    vals: Vec<(&'static str, String)>,
}

impl Fill {
    pub fn new() -> Fill {
        Fill::default()
    }
    pub fn set(mut self, k: &'static str, v: impl Into<String>) -> Fill {
        let v = v.into();
        if !v.is_empty() {
            self.vals.retain(|x| x.0 != k);
            self.vals.push((k, v));
        }
        self
    }
    pub fn get(&self, k: &str) -> Option<&str> {
        self.vals.iter().find(|x| x.0 == k).map(|x| x.1.as_str())
    }
}

/// The placeholders a template names.
fn names(t: &str) -> impl Iterator<Item = &str> {
    t.split('{').skip(1).filter_map(|s| s.split_once('}').map(|x| x.0))
}

/// A word from `words.allit.<letter>` for the first letter of `name`.
fn allit(rng: &mut Rng, name: &str, year: i32) -> Option<String> {
    let c = name.chars().find(|c| c.is_ascii_alphabetic())?.to_ascii_lowercase();
    let key = format!("words.allit.{c}");
    let lines = BANK.keys.get(key.as_str())?;
    let ok: Vec<&Line> = lines.iter().filter(|l| year >= l.from && year <= l.to).collect();
    (!ok.is_empty()).then(|| ok[(rng.next() * ok.len() as f64) as usize % ok.len()].text.to_string())
}

/// Fills one template. None when it names a value that isn't there.
fn fill_one(t: &str, f: &Fill, rng: &mut Rng, year: i32) -> Option<String> {
    let mut out = String::with_capacity(t.len() + 16);
    let mut rest = t;
    while let Some(k) = rest.find('{') {
        out.push_str(&rest[..k]);
        let after = &rest[k + 1..];
        let end = after.find('}')?;
        let name = &after[..end];
        let v = match name {
            "allit_first" => allit(rng, f.get("first")?, year)?,
            "allit_last" => allit(rng, f.get("last")?, year)?,
            _ => f.get(name)?.to_string(),
        };
        out.push_str(&v);
        rest = &after[end + 1..];
        // A count of one takes the singular: "{days} days" reads "1 day".
        if v == "1"
            && let Some(&(many, one)) = COUNT_NOUNS.iter().find(|x| {
                rest.strip_prefix(' ')
                    .and_then(|r| r.strip_prefix(x.0))
                    .is_some_and(|r| !r.starts_with(|c: char| c.is_alphanumeric()))
            })
        {
            out.push(' ');
            out.push_str(one);
            rest = &rest[1 + many.len()..];
        }
    }
    out.push_str(rest);
    Some(out)
}

/// One template filled, for the tests.
#[cfg(test)]
pub(crate) fn fill_for_test(t: &str, f: &Fill) -> Option<String> {
    fill_one(t, f, &mut Rng::new(1u32), 2026)
}

/// The nouns the banks count with a number placeholder, plural and singular.
const COUNT_NOUNS: [(&str, &str); 16] = [
    ("days", "day"),
    ("weeks", "week"),
    ("years", "year"),
    ("seasons", "season"),
    ("points", "point"),
    ("yards", "yard"),
    ("wins", "win"),
    ("losses", "loss"),
    ("titles", "title"),
    ("times", "time"),
    ("chances", "chance"),
    ("recruits", "recruit"),
    ("prospects", "prospect"),
    ("coaches", "coach"),
    ("teams", "team"),
    ("diamonds", "diamond"),
];

impl Bank {
    /// Whether a name is one of the real legends the game never reuses.
    pub fn blocked(&self, name: &str) -> bool {
        let n = name.to_lowercase();
        let bare = n.trim_start_matches("the ").trim();
        self.blocked.iter().any(|b| {
            let b = b.trim_start_matches("the ").trim();
            b == bare
        })
    }

    /// Every template of a key usable in `year` whose values are all given.
    fn usable(&self, key: &str, f: &Fill, year: i32) -> Vec<&'static str> {
        let Some(v) = self.keys.get(key) else { return Vec::new() };
        v.iter()
            .filter(|l| year >= l.from && year <= l.to)
            .filter(|l| names(l.text).all(|n| matches!(n, "allit_first" | "allit_last") || f.get(n).is_some()))
            .filter(|l| !names(l.text).any(|n| n == "allit_first") || f.get("first").is_some())
            .filter(|l| !names(l.text).any(|n| n == "allit_last") || f.get("last").is_some())
            .map(|l| l.text)
            .collect()
    }
}

/// Recently used lines per key, so a line rests before it comes back.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Recent {
    used: Vec<(u32, u64)>,
}

fn hash(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// How many lines a key remembers, against its size.
fn rest_len(n: usize) -> usize {
    (n * 2 / 3).clamp(0, 40)
}

impl Recent {
    /// A filled line from `key`, avoiding the ones used lately. None when
    /// the key has nothing usable.
    pub(crate) fn line(&mut self, rng: &mut Rng, key: &str, f: &Fill, year: i32) -> Option<String> {
        self.line_avoiding(rng, key, f, year, &[])
    }

    /// As `line`, and never a line one of `said` already starts with (a
    /// story's own beats: one story doesn't hear the same line twice).
    /// None when every usable line was said.
    pub(crate) fn line_avoiding(
        &mut self,
        rng: &mut Rng,
        key: &str,
        f: &Fill,
        year: i32,
        said: &[&str],
    ) -> Option<String> {
        let all = BANK.usable(key, f, year);
        if all.is_empty() {
            return None;
        }
        let kh = hash(key) as u32;
        let rest = rest_len(all.len());
        let mut recent: Vec<u64> = self.used.iter().filter(|x| x.0 == kh).map(|x| x.1).collect();
        if recent.len() > rest {
            recent.drain(..recent.len() - rest);
        }
        let (fresh, rested): (Vec<&&str>, Vec<&&str>) = all.iter().partition(|t| !recent.contains(&hash(t)));
        // The rested lines first; the recent ones only when there are none,
        // or a story said every rested one.
        let pools = if fresh.is_empty() { [rested, Vec::new()] } else { [fresh, rested] };
        for mut pool in pools {
            // A few tries: a template can still fail on an alliteration list
            // with nothing for its letter.
            let mut misses = 0;
            while !pool.is_empty() && misses < 6 {
                let k = (rng.next() * pool.len() as f64) as usize % pool.len();
                let t = pool[k];
                let Some(s) = fill_one(t, f, rng, year) else {
                    misses += 1;
                    continue;
                };
                let s = tidy(&s);
                if said.iter().any(|x| x.starts_with(s.as_str())) {
                    pool.swap_remove(k);
                    continue;
                }
                self.used.push((kh, hash(t)));
                let cap = 4000;
                if self.used.len() > cap {
                    self.used.drain(..self.used.len() - cap);
                }
                return Some(s);
            }
            if said.is_empty() {
                break;
            }
        }
        None
    }
}

/// Small clean-ups after filling: doubled spaces, a sentence's first
/// letter, "Jr.." at the end of a sentence.
pub(crate) fn tidy(s: &str) -> String {
    let mut t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    t = t.replace("Jr..", "Jr.").replace("Sr..", "Sr.").replace(" ,", ",").replace(" .", ".");
    // A name ending in s takes the apostrophe alone (AP style): "Illinois'".
    if t.contains("s's") {
        t = t
            .split(' ')
            .map(|word| match word.find("s's") {
                Some(k) if word.starts_with(|c: char| c.is_uppercase()) => {
                    format!("{}s'{}", &word[..k], &word[k + 3..])
                }
                _ => word.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" ");
    }
    let mut c = t.chars();
    match c.next() {
        Some(f) if f.is_lowercase() && !t.starts_with("the ") || f.is_lowercase() => {
            f.to_uppercase().collect::<String>() + c.as_str()
        }
        _ => t,
    }
}

/// A plain word from a word list key (no placeholders needed beyond `f`).
pub(crate) fn word(rng: &mut Rng, key: &str, f: &Fill, year: i32) -> Option<String> {
    let all = BANK.usable(key, f, year);
    if all.is_empty() {
        return None;
    }
    let t = all[(rng.next() * all.len() as f64) as usize % all.len()];
    fill_one(t, f, rng, year)
}

#[cfg(test)]
pub(crate) fn key_count(key: &str) -> usize {
    BANK.keys.get(key).map_or(0, |v| v.len())
}
