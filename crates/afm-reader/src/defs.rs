//! What the book works out: every check against real (`CHECKS`), the
//! distributions (`METRICS`), the consistency checks (`RULES`) and the
//! outliers (`OUTLIERS`). Each is SQL over the book's tables, written once
//! here so the app, `afm_read` and the threads all read the same numbers.
//!
//! Column names are the engine's field names, nested ones joined by `__`
//! (`player__position`); `x_` columns are what the reader works out
//! (`x_overall`). `afm_read <book> table <name>` lists a table's columns.

/// One check against real.
pub struct Check {
    pub id: &'static str,
    /// The topic page it belongs to (`pages::PAGES`).
    pub page: &'static str,
    /// Inventory areas.
    pub areas: &'static str,
    pub title: &'static str,
    pub unit: &'static str,
    /// SQL giving one number: the world's value.
    pub world: &'static str,
    /// The world's value times this (shares as percentages).
    pub scale: f64,
    /// SQL giving (year, value) rows, the value before `scale`.
    pub years: Option<&'static str>,
    /// SQL giving one line of text that names the facts behind the number.
    pub detail: Option<&'static str>,
    pub real: Real,
    /// near, off, floor (`marks.rs`).
    pub tol: (f64, f64, f64),
    /// What the book needs for it: "world file", "game log" or "world log".
    pub needs: &'static str,
    /// How the world's number is worked out, in words.
    pub how: &'static str,
}

/// Where a check's real figure comes from.
pub enum Real {
    /// A figure in the real index (`file:path`, `#a.b` into its value),
    /// times the scale.
    Key(&'static str, f64),
    /// A figure the index has no key for: value, source, years.
    Fixed(f64, &'static str, &'static str),
    /// A real figure from memory, still to be sourced.
    Memory(f64, &'static str),
    /// No real figure exists; why.
    None(&'static str),
}

/// A distribution, worked out by year and by group.
pub struct Metric {
    pub id: &'static str,
    pub page: &'static str,
    pub areas: &'static str,
    pub title: &'static str,
    pub unit: &'static str,
    /// SQL giving rows of (year, group kind, group, value); year may be
    /// null for a measure without years.
    pub sql: &'static str,
    pub needs: &'static str,
}

/// A consistency check between stores (inventory T11).
pub struct Rule {
    pub id: &'static str,
    pub areas: &'static str,
    pub title: &'static str,
    /// SQL giving how many things were checked.
    pub checked: &'static str,
    /// SQL giving each failure: (what, id, year, facts).
    pub failures: &'static str,
    pub needs: &'static str,
    pub why: &'static str,
}

/// Rare values worth a look: the top of a measure, each with its
/// percentile against the world.
pub struct Outlier {
    pub id: &'static str,
    pub page: &'static str,
    pub title: &'static str,
    /// SQL giving (what, id, year, value, words) for every row measured.
    pub sql: &'static str,
    /// Shown when above this percentile (0 to 1); `low` shows the bottom.
    pub above: f64,
    pub low: bool,
    pub needs: &'static str,
}

/// Every check: the book's own (`defs_list`) and each page lane's.
pub fn checks() -> impl Iterator<Item = &'static Check> {
    crate::defs_list::CHECKS
        .iter()
        .chain(crate::defs_people::CHECKS)
        .chain(crate::defs_games::CHECKS)
        .chain(crate::defs_nfl::CHECKS)
        .chain(crate::defs_world::CHECKS)
}

/// Every distribution.
pub fn metrics() -> impl Iterator<Item = &'static Metric> {
    crate::defs_list::METRICS
        .iter()
        .chain(crate::defs_people::METRICS)
        .chain(crate::defs_games::METRICS)
        .chain(crate::defs_nfl::METRICS)
        .chain(crate::defs_world::METRICS)
}

/// Every consistency check.
pub fn rules() -> impl Iterator<Item = &'static Rule> {
    crate::defs_list::RULES
        .iter()
        .chain(crate::defs_people::RULES)
        .chain(crate::defs_games::RULES)
        .chain(crate::defs_nfl::RULES)
        .chain(crate::defs_world::RULES)
}

/// Every column the defs read that a small or early world may lack.
pub fn columns() -> impl Iterator<Item = &'static (&'static str, &'static str)> {
    crate::defs_list::COLUMNS
        .iter()
        .chain(crate::defs_people::COLUMNS)
        .chain(crate::defs_games::COLUMNS)
        .chain(crate::defs_nfl::COLUMNS)
        .chain(crate::defs_world::COLUMNS)
}

/// Tables the lane R8 pages read that an older or smaller world may not
/// have at all: the book makes them empty so a page shows no rows, not an
/// error.
pub fn makes_missing(table: &str) -> bool {
    crate::defs_games::COLUMNS
        .iter()
        .chain(crate::defs_nfl::COLUMNS)
        .chain(crate::defs_world::COLUMNS)
        .any(|(t, _)| *t == table)
}

/// Every outlier list.
pub fn outliers() -> impl Iterator<Item = &'static Outlier> {
    crate::defs_list::OUTLIERS
        .iter()
        .chain(crate::defs_people::OUTLIERS)
        .chain(crate::defs_games::OUTLIERS)
        .chain(crate::defs_nfl::OUTLIERS)
        .chain(crate::defs_world::OUTLIERS)
}
