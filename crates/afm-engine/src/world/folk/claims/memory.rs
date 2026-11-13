//! What you have already talked through with each person (talk facts
//! design, rule 6 and section 5, "Follow-ups and memory"): kept for a
//! season, with the day.

use super::kinds::Kind;
use super::sheet::{Point, Tier};
use crate::calendar::Date;
use serde::{Deserialize, Serialize};

/// The talked-through points are kept this long (the design: "for a
/// season").
/// Proposed, not confirmed.
pub const KEEP_DAYS: i32 = 365;
/// Coming back to a point that landed adds this share of it, then this
/// share again (half, a quarter). Proposed, not confirmed.
pub const AGAIN: f32 = 0.5;

/// One point you made to one person.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Whom you said it to (player id).
    pub who: u32,
    pub kind: Kind,
    /// The day of the fact it was about (`Point::fact`): a second arrest
    /// is a new point.
    pub of: Option<Date>,
    /// The point's topic, when one day holds two facts of a kind (two
    /// promises due the same day).
    pub what: &'static str,
    /// The last day you made it.
    pub on: Date,
    /// How many talks it has come up in.
    pub times: u8,
    /// Whether it landed the last time.
    pub landed: bool,
    /// Your proof's tier that time (`Tier::rank`; 0 for a point without
    /// proof).
    pub tier: u8,
    /// What you did with it, as the tree names it (a move's key or a
    /// pick's code): "Repeat praise for the same game and he says so."
    pub said: String,
}

/// The rows as a save holds them: the kind by its key, so a save written
/// by a newer catalogue still opens (a key this build doesn't know drops
/// that one row).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct Saved {
    who: u32,
    kind: String,
    of: Option<Date>,
    #[serde(skip_serializing_if = "String::is_empty")]
    what: String,
    on: Date,
    times: u8,
    landed: bool,
    tier: u8,
    said: String,
}

/// What you have talked through, newest last.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(from = "Vec<Saved>", into = "Vec<Saved>")]
pub struct Talked {
    rows: Vec<Row>,
}

impl From<Vec<Saved>> for Talked {
    fn from(v: Vec<Saved>) -> Talked {
        Talked {
            rows: v
                .into_iter()
                .filter_map(|s| {
                    Some(Row {
                        who: s.who,
                        kind: Kind::from_key(&s.kind)?,
                        of: s.of,
                        what: topic_of(&s.what),
                        on: s.on,
                        times: s.times,
                        landed: s.landed,
                        tier: s.tier,
                        said: s.said,
                    })
                })
                .collect(),
        }
    }
}

impl From<Talked> for Vec<Saved> {
    fn from(t: Talked) -> Vec<Saved> {
        t.rows
            .into_iter()
            .map(|r| Saved {
                who: r.who,
                kind: r.kind.key().to_string(),
                of: r.of,
                what: r.what.to_string(),
                on: r.on,
                times: r.times,
                landed: r.landed,
                tier: r.tier,
                said: r.said,
            })
            .collect()
    }
}

/// A saved topic as the one a point carries (an unknown one is kept as
/// none, so the row still reads).
fn topic_of(s: &str) -> &'static str {
    super::player::PROMISE_TOPICS.iter().copied().find(|&t| t == s).unwrap_or("")
}

impl Talked {
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The row for a point: the same person, kind and fact.
    pub fn row(&self, who: u32, p: &Point) -> Option<&Row> {
        let (of, what) = p.fact();
        self.rows.iter().rev().find(|r| r.who == who && r.kind == p.kind && r.of == of && r.what == what)
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// How many talks a point has come up in, in the last season.
    pub fn times(&self, today: Date, who: u32, p: &Point) -> u8 {
        self.row(who, p).filter(|r| r.on.days_to(today) <= KEEP_DAYS).map_or(0, |r| r.times)
    }

    /// Everything you have talked through with one person, newest first.
    pub fn with(&self, who: u32) -> impl Iterator<Item = &Row> {
        self.rows.iter().rev().filter(move |r| r.who == who)
    }

    /// Keeps a point you made today: once a talk, however often it came up
    /// in it (rule 6: a point counts once in a talk). Rows older than a
    /// season go. `said` is what you did with it, as the tree names it.
    pub fn note(&mut self, today: Date, who: u32, p: &Point, landed: bool, said: &str) {
        self.rows.retain(|r| r.on.days_to(today) <= KEEP_DAYS);
        let (kind, (of, what)) = (p.kind, p.fact());
        let tier = p.tier().map_or(0, Tier::rank);
        match self.rows.iter_mut().rev().find(|r| r.who == who && r.kind == kind && r.of == of && r.what == what) {
            // Said again in the same talk: it landed if it landed either
            // time, at the better proof.
            Some(r) if r.on == today => {
                r.landed |= landed;
                r.tier = r.tier.max(tier);
                r.said = said.to_string();
            }
            Some(r) => {
                r.times = r.times.saturating_add(1);
                r.on = today;
                r.landed = landed;
                r.tier = tier;
                r.said = said.to_string();
            }
            None => {
                self.rows.push(Row { who, kind, of, what, on: today, times: 1, landed, tier, said: said.to_string() })
            }
        }
    }

    /// What a point adds when you make it today (rule 6; section 6, "How
    /// much it counts"): all of it the first time; nothing more in the same
    /// talk; in a later talk half, then a quarter, when it landed and (for
    /// a point with proof) your proof was strong or elite; nothing when it
    /// didn't land. A fact that got better since counts in full.
    pub fn again(&self, today: Date, who: u32, p: &Point) -> f32 {
        let Some(r) = self.row(who, p) else { return 1.0 };
        if r.on.days_to(today) > KEEP_DAYS {
            return 1.0;
        }
        if p.tier().map_or(0, Tier::rank) > r.tier {
            return 1.0;
        }
        if r.on == today || !r.landed || (r.tier != 0 && r.tier < Tier::Strong.rank()) {
            return 0.0;
        }
        AGAIN.powi(r.times as i32)
    }

    /// Whether you have already made this point to him (he says he
    /// remembers it).
    pub fn remembers(&self, today: Date, who: u32, p: &Point) -> bool {
        self.row(who, p).is_some_and(|r| r.on.days_to(today) <= KEEP_DAYS)
    }

    /// Drops a person's rows (he left football).
    pub fn forget(&mut self, who: u32) {
        self.rows.retain(|r| r.who != who);
    }
}
