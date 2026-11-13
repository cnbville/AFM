//! What the calls cost: the price table, and the spend book by calendar
//! month with the game's $50 cap (live talks, Cain 2026-11-07).
//!
//! Money is counted in billionths of a dollar ("nano"). A price in dollars
//! per million tokens times 1,000 is nano per token, so every cost is a
//! whole number and sums never drift.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::client::Usage;

/// The monthly cap, in dollars and in nano (Cain 2026-11-07 14:22 UTC: "a
/// monthly cap of $50"; confirmed).
pub const CAP_USD: f64 = 50.0;
pub const CAP_NANO: u64 = 50_000_000_000;

/// How many months the book keeps (proposed, not confirmed).
const KEEP_MONTHS: usize = 24;

/// Nano per token for each kind of token.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rates {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    /// Writing to the 5-minute cache: 1.25 times the input rate.
    pub write_5m: u64,
    /// Writing to the 1-hour cache: 2 times the input rate.
    pub write_1h: u64,
}

/// A model's prices. A prompt over `LONG_PROMPT` tokens is billed on the
/// second card.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Price {
    pub short: Rates,
    pub long: Rates,
}

/// Prompts above this many tokens cost more: the real price card for Haiku
/// 5.5 (source: the claude-api skill, $0.10/$0.50 up to 100K, then $0.50/$2.50).
pub const LONG_PROMPT: u64 = 100_000;

impl Price {
    /// The table. A model that is not in it has no price and is never called
    /// (the cap could not count it). Source: the `claude-api` skill's model
    /// table and its Haiku 5.5 notes, read 2026-11-13, and the project's
    /// proposal ("Model and cost"). Sonnet 5.5 is listed for the build-side
    /// tools (the playtester, "Ask the world"); the game itself uses Haiku.
    pub fn of(model: &str) -> Option<Price> {
        match model {
            "claude-haiku-5-5" => Some(Price {
                short: Rates { input: 100, output: 500, cache_read: 10, write_5m: 125, write_1h: 200 },
                long: Rates { input: 500, output: 2500, cache_read: 50, write_5m: 625, write_1h: 1000 },
            }),
            "claude-sonnet-5-5" => {
                let r = Rates { input: 2000, output: 10000, cache_read: 100, write_5m: 2500, write_1h: 4000 };
                Some(Price { short: r, long: r })
            }
            _ => None,
        }
    }

    /// What a reply's usage cost, in nano.
    pub fn cost(&self, u: &Usage) -> u64 {
        let r = if u.prompt() > LONG_PROMPT { self.long } else { self.short };
        u.input * r.input
            + u.output * r.output
            + u.cache_read * r.cache_read
            + u.cache_write_5m * r.write_5m
            + u.cache_write_1h * r.write_1h
    }
}

/// Dollars, for the screen.
pub fn usd(nano: u64) -> f64 {
    nano as f64 / 1e9
}

/// One calendar month of spending. A field missing from the file reads as zero,
/// so a book written by an older or newer game still reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MonthBook {
    pub nano: u64,
    pub calls: u64,
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct File {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    months: BTreeMap<String, MonthBook>,
}

/// The book: every month's spending, kept in `<game folder>/ai-spend.json`.
#[derive(Debug)]
pub struct Spend {
    path: Option<PathBuf>,
    months: BTreeMap<String, MonthBook>,
    cap: u64,
    /// Set when the file could not be read and was set aside.
    pub note: Option<String>,
}

impl Spend {
    /// A book kept in memory only.
    pub fn memory() -> Spend {
        Spend { path: None, months: BTreeMap::new(), cap: CAP_NANO, note: None }
    }

    /// Opens the book at `path`. A missing file is a new book. A file that
    /// cannot be read is kept beside it as `.bad` and the book starts again
    /// from zero, with a note (the key's own limit in the Console is the
    /// backstop).
    pub fn open(path: PathBuf) -> Spend {
        let mut s = Spend { path: Some(path.clone()), months: BTreeMap::new(), cap: CAP_NANO, note: None };
        match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<File>(&bytes) {
                Ok(f) => s.months = f.months,
                Err(_) => {
                    let _ = std::fs::rename(&path, path.with_extension("bad"));
                    s.note =
                        Some("The spend file could not be read. It was set aside and counting started again.".into());
                }
            },
            Err(_) => {}
        }
        s
    }

    pub fn with_cap(mut self, nano: u64) -> Spend {
        self.cap = nano;
        self
    }

    pub fn cap(&self) -> u64 {
        self.cap
    }

    pub fn book(&self, month: &str) -> MonthBook {
        self.months.get(month).cloned().unwrap_or_default()
    }

    pub fn spent(&self, month: &str) -> u64 {
        self.months.get(month).map_or(0, |m| m.nano)
    }

    pub fn cap_reached(&self, month: &str) -> bool {
        self.spent(month) >= self.cap
    }

    /// Adds one reply's cost to the month and writes the book.
    pub fn add(&mut self, month: &str, u: &Usage, nano: u64) {
        let m = self.months.entry(month.to_string()).or_default();
        m.nano += nano;
        m.calls += 1;
        m.input += u.input;
        m.output += u.output;
        m.cache_read += u.cache_read;
        m.cache_write += u.cache_write();
        while self.months.len() > KEEP_MONTHS {
            let first = self.months.keys().next().cloned();
            match first {
                Some(k) => self.months.remove(&k),
                None => break,
            };
        }
        self.save();
    }

    /// Written to `.partial` and renamed, so a stop in the middle leaves the
    /// old book. A write that fails is left: the call is already paid for
    /// and the total stays right in memory.
    fn save(&self) {
        let Some(path) = &self.path else { return };
        let file = File { version: 1, months: self.months.clone() };
        let Ok(text) = serde_json::to_string_pretty(&file) else { return };
        let tmp = path.with_extension("partial");
        let _ = std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path));
    }
}

/// This month in UTC, as `2026-11`.
pub fn month_now() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    month_of(secs)
}

/// The month of a Unix time, in UTC.
pub fn month_of(secs: u64) -> String {
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's method).
    let z = (secs / 86_400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(input: u64, output: u64) -> Usage {
        Usage { input, output, ..Usage::default() }
    }

    #[test]
    fn haiku_costs_match_the_price_table() {
        let p = Price::of("claude-haiku-5-5").unwrap();
        // 100,000 input tokens (the top of the short card) at $0.10 a million is a cent.
        assert_eq!(p.cost(&usage(100_000, 0)), 10_000_000);
        assert_eq!(p.cost(&usage(0, 100_000)), 50_000_000);
        let all = Usage {
            input: 1_000_000,
            output: 1_000_000,
            cache_read: 1_000_000,
            cache_write_5m: 1_000_000,
            cache_write_1h: 1_000_000,
        };
        // Five million prompt tokens is a long prompt, so every kind is on the second card.
        assert!(all.prompt() > LONG_PROMPT);
        assert_eq!(p.cost(&all), (500 + 2500 + 50 + 625 + 1000) * 1_000_000);
        // The proposal's reader: about 1,800 tokens in and 100 out, $0.00023 a call.
        let reader = p.cost(&usage(1_800, 100));
        assert_eq!(reader, 1_800 * 100 + 100 * 500);
        assert!((usd(reader) - 0.00023).abs() < 1e-9);
        // Cached rules: the read costs a tenth of the input.
        assert_eq!(p.cost(&Usage { cache_read: 100_000, ..Usage::default() }), 1_000_000);
    }

    #[test]
    fn a_prompt_over_100k_tokens_is_billed_on_the_second_card() {
        let p = Price::of("claude-haiku-5-5").unwrap();
        assert_eq!(p.cost(&usage(100_000, 0)), 100_000 * 100);
        assert_eq!(p.cost(&usage(100_001, 0)), 100_001 * 500);
    }

    #[test]
    fn a_model_with_no_price_is_not_in_the_table() {
        assert!(Price::of("claude-haiku-4-5").is_none());
        assert!(Price::of("claude-sonnet-5-5").is_some());
    }

    #[test]
    fn months_come_from_unix_time_in_utc() {
        assert_eq!(month_of(0), "1970-01");
        assert_eq!(month_of(1_700_000_000), "2023-11");
        // 2026-11-13 02:45 UTC, and the last second of October 2026.
        assert_eq!(month_of(1_794_537_900), "2026-11");
        assert_eq!(month_of(1_793_491_199), "2026-10");
        assert_eq!(month_of(1_793_491_200), "2026-11");
        // A leap day.
        assert_eq!(month_of(1_709_164_800), "2024-02");
    }

    #[test]
    fn the_cap_and_the_months_reset() {
        let mut s = Spend::memory().with_cap(1_000);
        s.add("2026-10", &usage(1, 1), 999);
        assert!(!s.cap_reached("2026-10"));
        s.add("2026-10", &usage(1, 1), 1);
        assert!(s.cap_reached("2026-10"));
        // A new month starts at zero.
        assert!(!s.cap_reached("2026-11"));
        assert_eq!(s.spent("2026-11"), 0);
        assert_eq!(s.book("2026-10").calls, 2);
        assert_eq!(s.cap(), 1_000);
    }

    #[test]
    fn the_book_is_kept_in_a_file_and_read_back() {
        let dir = std::env::temp_dir().join(format!("afm-ai-spend-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ai-spend.json");
        let mut s = Spend::open(path.clone());
        s.add("2026-11", &usage(10, 5), 1_234);
        assert!(path.exists() && !path.with_extension("partial").exists());
        let back = Spend::open(path.clone());
        assert_eq!(back.spent("2026-11"), 1_234);
        assert_eq!(back.book("2026-11").output, 5);
        // A broken file is set aside, and counting starts again.
        std::fs::write(&path, "{ not json").unwrap();
        let fresh = Spend::open(path.clone());
        assert_eq!(fresh.spent("2026-11"), 0);
        assert!(fresh.note.is_some());
        assert!(path.with_extension("bad").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_book_keeps_two_years() {
        let mut s = Spend::memory();
        for m in 0..30u32 {
            s.add(&format!("{:04}-{:02}", 2020 + m / 12, m % 12 + 1), &usage(1, 1), 1);
        }
        assert_eq!(s.months.len(), KEEP_MONTHS);
        assert_eq!(s.spent("2020-01"), 0);
        assert_eq!(s.spent("2022-06"), 1);
    }
}
