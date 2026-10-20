//! Events on the calendar: what happens on a day, at which level, under which
//! rule, how loud it is (topic 8) and whether it waits on a decision.
//!
//! Every system posts into one calendar. The calendar keeps the events of the
//! current football year and a trimmed record of past years (the loud ones),
//! which the "on this day" anniversaries read (A42).

use super::date::{Date, Moment};
use std::collections::BTreeMap;

/// How loud a moment is (topic 8): small is a news line, medium an inbox
/// message, big its own screen (Continue stops on it), huge the full show
/// that neither Continue nor holiday mode ever skips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
pub enum Loudness {
    Small,
    Medium,
    Big,
    Huge,
}

impl Loudness {
    pub fn name(self) -> &'static str {
        match self {
            Loudness::Small => "small",
            Loudness::Medium => "medium",
            Loudness::Big => "big",
            Loudness::Huge => "huge",
        }
    }
}

/// Which level an event belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Level {
    Nfl,
    College,
    HighSchool,
    /// Recruiting crosses high school and college.
    Recruiting,
    /// Weather and the world around football.
    World,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::Nfl => "NFL",
            Level::College => "college",
            Level::HighSchool => "high school",
            Level::Recruiting => "recruiting",
            Level::World => "world",
        }
    }
}

/// What an event is, for filters, trials and the future screens.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Kind {
    /// A fixed date in a level's year (the combine, the draft, signing day).
    Date,
    /// A window opening or closing (tag window, portal, a recruiting period).
    Window,
    /// A deadline someone must act by.
    Deadline,
    Game,
    Poll,
    Award,
    Ceremony,
    Coaching,
    Transfer,
    DraftChoice,
    DraftPick,
    Signing,
    Commitment,
    /// A scholarship offer (part B 34).
    Offer,
    Visit,
    Practice,
    Academic,
    Weather,
    RuleChange,
    Anniversary,
    Whisper,
    /// A dated slot another section fills later (contracts, owners, people).
    Slot,
    /// A daily step in the week's rhythm (film Monday, injury report Wednesday).
    Rhythm,
    Breakout,
    News,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub when: Moment,
    pub level: Level,
    pub kind: Kind,
    pub loud: Loudness,
    /// Waits on a decision from whoever runs the team it is about.
    pub needs_decision: bool,
    /// The rule that allows it ("CBA: tag window", "NCAA: dead period").
    #[serde(deserialize_with = "crate::save::leaked")]
    pub rule: crate::save::Text,
    /// An NFL team (index), college program or high school it is about.
    pub nfl_team: Option<usize>,
    pub program: Option<usize>,
    pub text: String,
    /// Remembered on its anniversaries (A42): title games, the Heisman, the
    /// first pick.
    pub memorable: bool,
    /// The football year it is filed under when that is not the calendar's
    /// own: a February signing day after the Super Bowl belongs to the
    /// class's season, like that day's date line.
    pub season: Option<i32>,
}

impl Event {
    pub fn new(date: Date, level: Level, kind: Kind, loud: Loudness, text: impl Into<String>) -> Event {
        Event {
            when: Moment { date, minute: 9 * 60 },
            level,
            kind,
            loud,
            needs_decision: false,
            rule: "",
            nfl_team: None,
            program: None,
            text: text.into(),
            memorable: false,
            season: None,
        }
    }
    pub fn memorable(mut self) -> Event {
        self.memorable = true;
        self
    }
    pub fn at(mut self, when: Moment) -> Event {
        self.when = when;
        self
    }
    pub fn rule(mut self, rule: &'static str) -> Event {
        self.rule = rule;
        self
    }
    pub fn decision(mut self) -> Event {
        self.needs_decision = true;
        self
    }
    pub fn program(mut self, p: usize) -> Event {
        self.program = Some(p);
        self
    }
    pub fn team(mut self, t: usize) -> Event {
        self.nfl_team = Some(t);
        self
    }
}

/// An answer someone is waiting on (topic 9): an agent's reply, a trade
/// physical, a coaching search. It comes due at `due`, and while it waits it
/// can fall apart.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pending {
    pub id: u64,
    pub opened: Moment,
    pub due: Moment,
    /// Chance per day that the deal dies while it waits.
    pub fall_apart_per_day: f64,
    pub what: String,
    pub nfl_team: Option<usize>,
    pub program: Option<usize>,
}

/// How a pending answer came out.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Outcome {
    Answered,
    FellApart,
}

/// The calendar: the year's events, pending answers and a record of loud
/// moments from past years.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Calendar {
    /// This football year's events by date, in the order they were posted.
    pub events: BTreeMap<Date, Vec<Event>>,
    pub pending: Vec<Pending>,
    pub resolved: Vec<(Pending, Moment, Outcome)>,
    /// Big and huge moments from past years, kept for anniversaries (A42).
    pub history: Vec<Event>,
    /// Count of every event posted by (level, kind), across all years, for trials.
    #[serde(deserialize_with = "crate::save::leaked")]
    pub counts: BTreeMap<(&'static str, String), u64>,
    /// While set, what is posted belongs to this football year, not the
    /// calendar's own (a signing day that falls after the Super Bowl). It
    /// still shows on its day.
    pub filing: Option<i32>,
    next_id: u64,
}

impl Calendar {
    pub fn post(&mut self, mut e: Event) {
        e.season = e.season.or(self.filing);
        *self.counts.entry((e.level.name(), format!("{:?}", e.kind))).or_default() += 1;
        self.events.entry(e.when.date).or_default().push(e);
    }

    pub fn on(&self, d: Date) -> &[Event] {
        self.events.get(&d).map_or(&[], |v| v.as_slice())
    }

    /// The day's events, earliest first (the hour-by-hour days of topic 1.5).
    pub fn hour_by_hour(&self, d: Date) -> Vec<&Event> {
        let mut v: Vec<&Event> = self.on(d).iter().collect();
        v.sort_by_key(|e| e.when.minute);
        v
    }

    /// Opens a pending answer and returns its id.
    pub fn wait_for(&mut self, mut p: Pending) -> u64 {
        self.next_id += 1;
        p.id = self.next_id;
        let id = p.id;
        self.pending.push(p);
        id
    }

    /// Resolves every pending answer due by `now`; each day it waited it
    /// could fall apart (`roll` returns a uniform draw in [0, 1)).
    pub fn resolve_due(&mut self, now: Moment, roll: &mut dyn FnMut() -> f64) -> Vec<(Pending, Outcome)> {
        let mut out = Vec::new();
        let mut keep = Vec::with_capacity(self.pending.len());
        for p in std::mem::take(&mut self.pending) {
            if p.due > now {
                keep.push(p);
                continue;
            }
            let days = p.opened.date.days_to(p.due.date).max(1) as f64;
            let survive = (1.0 - p.fall_apart_per_day).powf(days);
            let outcome = if roll() < survive { Outcome::Answered } else { Outcome::FellApart };
            self.resolved.push((p.clone(), now, outcome.clone()));
            out.push((p, outcome));
        }
        self.pending = keep;
        out
    }

    /// Ends a football year: loud moments move to history, the rest is dropped.
    /// Events dated `keep_from` or later stay: a signing day after the
    /// Super Bowl, or news posted ahead, belongs to the new year (part C 1).
    pub fn close_year(&mut self, keep_from: Date) {
        let ahead = self.events.split_off(&keep_from);
        for (_, list) in std::mem::replace(&mut self.events, ahead) {
            for e in list {
                if e.loud >= Loudness::Big {
                    self.history.push(e);
                }
            }
        }
        if self.resolved.len() > 2000 {
            let cut = self.resolved.len() - 2000;
            self.resolved.drain(..cut);
        }
    }

    /// Every event of the current year, in date order.
    pub fn all(&self) -> impl Iterator<Item = &Event> {
        self.events.values().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_answers_come_due_and_can_fall_apart() {
        let mut c = Calendar::default();
        let d = Date::ymd(2026, 3, 11);
        for k in 0..200 {
            c.wait_for(Pending {
                id: 0,
                opened: Moment::at(d, 16, 0),
                due: Moment::at(d.add(3), 10, 0),
                fall_apart_per_day: 0.1,
                what: format!("offer {k}"),
                nfl_team: Some(0),
                program: None,
            });
        }
        let mut x = 0.0f64;
        let mut roll = || {
            x = (x + 0.618_034) % 1.0;
            x
        };
        assert!(c.resolve_due(Moment::at(d.add(1), 9, 0), &mut roll).is_empty());
        let done = c.resolve_due(Moment::at(d.add(3), 12, 0), &mut roll);
        assert_eq!(done.len(), 200);
        let died = done.iter().filter(|d| d.1 == Outcome::FellApart).count();
        // 1 - 0.9^3 = 27% die while waiting.
        assert!((35..80).contains(&died), "{died} fell apart");
    }
}
