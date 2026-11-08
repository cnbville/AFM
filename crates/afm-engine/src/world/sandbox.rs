//! Sandbox mode (Settings, a test switch for your own career): while it is
//! on, each of its switches lifts one of YOUR limits, so you can test a
//! screen or a system without the real rules in the way: watch film
//! endlessly, offer more NIL money than your budget, call recruits in a dead
//! period. The world never sees a lifted limit: the AI programs, the AI
//! clubs and every other person play by the real rules, exactly as before.
//!
//! It works on any save: a career from before sandbox mode loads with it
//! off (`#[serde(default)]`). The first time it goes on the career is marked
//! for good (`since`, on the save's header too), even after it goes off, the
//! way the no-fire override marks it. The no-fire override is one of the
//! switches; a save that had it on loads with sandbox mode on and only that
//! switch lifted (`after_load`).

use crate::calendar::Date;

use super::World;

/// One limit sandbox mode can lift.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Switch {
    /// No weekly cap on film hours: every tape, session and run-back fits.
    FilmHours,
    /// No tired eye, no wear from long weeks and never sent home to rest.
    FreshEye,
    /// The hours of your week outside film (one-on-ones, interviews,
    /// meetings, events, visits) never run out.
    YourHours,
    /// Your staff's days and calls never run out (evaluation days, staked
    /// calls).
    StaffTime,
    /// NIL offers past your NIL budget and the revenue-sharing cap.
    NilBudget,
    /// NIL Go clears every deal you make in full.
    NilGo,
    /// Money on your side never runs out: school and staff budgets,
    /// salary pools, favours that cost money.
    Money,
    /// Recruiting contact rules: calls, visits and home visits in any
    /// period, no visit counts, no cap on your own targets.
    ContactRules,
    /// Your roster and scholarship counts: sign past the limit.
    RosterLimits,
    /// Your NFL club signs, restructures and extends past the cap.
    SalaryCap,
    /// Practice plans past the hour, contact and day limits, with no
    /// violation.
    PracticeRules,
    /// Nothing ends your job (the no-fire override).
    NoFire,
}

impl Switch {
    pub const ALL: [Switch; 12] = [
        Switch::FilmHours,
        Switch::FreshEye,
        Switch::YourHours,
        Switch::StaffTime,
        Switch::NilBudget,
        Switch::NilGo,
        Switch::Money,
        Switch::ContactRules,
        Switch::RosterLimits,
        Switch::SalaryCap,
        Switch::PracticeRules,
        Switch::NoFire,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Switch::FilmHours => "film_hours",
            Switch::FreshEye => "fresh_eye",
            Switch::YourHours => "your_hours",
            Switch::StaffTime => "staff_time",
            Switch::NilBudget => "nil_budget",
            Switch::NilGo => "nil_go",
            Switch::Money => "money",
            Switch::ContactRules => "contact_rules",
            Switch::RosterLimits => "roster_limits",
            Switch::SalaryCap => "salary_cap",
            Switch::PracticeRules => "practice_rules",
            Switch::NoFire => "no_fire",
        }
    }

    pub fn from_key(k: &str) -> Option<Switch> {
        Switch::ALL.into_iter().find(|s| s.key() == k)
    }

    /// The switch's name in Settings.
    pub fn label(self) -> &'static str {
        match self {
            Switch::FilmHours => "Film hours",
            Switch::FreshEye => "Fresh eye",
            Switch::YourHours => "Your hours",
            Switch::StaffTime => "Staff time",
            Switch::NilBudget => "NIL budget",
            Switch::NilGo => "NIL Go",
            Switch::Money => "Money",
            Switch::ContactRules => "Contact rules",
            Switch::RosterLimits => "Roster limits",
            Switch::SalaryCap => "Salary cap",
            Switch::PracticeRules => "Practice rules",
            Switch::NoFire => "No-fire",
        }
    }

    /// What it lifts, in Settings' words.
    pub fn what(self) -> &'static str {
        match self {
            Switch::FilmHours => {
                "No weekly cap on film: every tape, session, run-back and game-plan study fits, and your hunt's games are always watched."
            }
            Switch::FreshEye => "No tired eye late in the day, no wear from long weeks, never sent home to rest.",
            Switch::YourHours => {
                "One-on-ones, meetings, interviews, calls, trips and school visits never fill your week, and every recruiting target gets your own time."
            }
            Switch::StaffTime => {
                "Your staff's evaluation days never run out, no cap on staked calls or the workout and recheck lists, and the self-scout works outside a bye."
            }
            Switch::NilBudget => {
                "Promises, deals, portal offers and your hunt's money go past what's left and past the revenue-sharing cap."
            }
            Switch::NilGo => "Your collective's deals clear NIL Go in full: nothing held back.",
            Switch::Money => {
                "The equipment budget, the booster club's gift to a feeder, scout and personnel posts, the undrafted pool and paid head starts never run out."
            }
            Switch::ContactRules => {
                "Visits, home visits and drop-ins in any period with no counts, evaluation trips any week, free-agent visits before the league year, and no cap on pitches or the visit and interview lists."
            }
            Switch::RosterLimits => {
                "Recruits past your class plan and roster limit, portal players past it, more than 53 kept, a bigger practice squad and every return from injured reserve."
            }
            Switch::SalaryCap => {
                "Your club signs, tags, tenders, claims, trades and restructures past the cap, and the league doesn't cut you down on opening day."
            }
            Switch::PracticeRules => {
                "Any week or template you write saves, and as head coach it runs as written and nobody opens a case."
            }
            Switch::NoFire => "Nothing ends your job. Your inbox still says each time it would have.",
        }
    }

    /// The group it sits in on Settings.
    pub fn group(self) -> &'static str {
        match self {
            Switch::FilmHours | Switch::FreshEye | Switch::YourHours | Switch::StaffTime => "Time",
            Switch::NilBudget | Switch::NilGo | Switch::Money | Switch::SalaryCap => "Money",
            Switch::ContactRules | Switch::RosterLimits | Switch::PracticeRules => "Rules",
            Switch::NoFire => "Your job",
        }
    }
}

/// Sandbox mode on your career.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Sandbox {
    /// Sandbox mode is on: the lifted switches below apply.
    #[serde(default)]
    pub on: bool,
    /// The day sandbox mode was first turned on: a career that ever used it
    /// stays marked, on its saves too.
    #[serde(default)]
    pub since: Option<Date>,
    /// The switches you lifted, by key (a key a later game drops is
    /// ignored, so a save never fails to load over one).
    #[serde(default)]
    pub lifted: Vec<String>,
    /// Set the first time sandbox mode goes on, when every switch is
    /// lifted; after that your own picks stay.
    #[serde(default)]
    pub picked: bool,
}

impl Sandbox {
    /// The switch is lifted and sandbox mode is on.
    pub fn lifts(&self, s: Switch) -> bool {
        self.on && self.lifted.iter().any(|k| k == s.key())
    }

    /// The switch is lifted, whether sandbox mode is on or not.
    pub fn picked(&self, s: Switch) -> bool {
        self.lifted.iter().any(|k| k == s.key())
    }
}

impl World {
    /// Is this limit of yours lifted right now? False without a career.
    pub fn sandbox(&self, s: Switch) -> bool {
        self.career.as_ref().is_some_and(|k| k.sandbox.lifts(s))
    }

    /// Turns sandbox mode on or off. The first time it goes on, the career
    /// is marked for good and every switch is lifted; you turn single ones
    /// off after that.
    pub fn set_sandbox(&mut self, on: bool) {
        let today = self.today;
        let Some(k) = self.career.as_mut() else { return };
        let b = &mut k.sandbox;
        b.on = on;
        if on {
            b.since.get_or_insert(today);
            if !b.picked {
                b.picked = true;
                b.lifted = Switch::ALL.iter().map(|s| s.key().to_string()).collect();
            }
        }
        k.no_fire = b.lifts(Switch::NoFire);
        if k.no_fire {
            k.no_fire_since.get_or_insert(today);
        }
    }

    /// Lifts one switch or puts its limit back. Lifting one turns sandbox
    /// mode on (and marks the career).
    pub fn set_sandbox_switch(&mut self, s: Switch, lift: bool) {
        let today = self.today;
        let Some(k) = self.career.as_mut() else { return };
        let b = &mut k.sandbox;
        b.picked = true;
        b.lifted.retain(|x| x != s.key());
        if lift {
            b.lifted.push(s.key().to_string());
            b.on = true;
            b.since.get_or_insert(today);
        }
        k.no_fire = b.lifts(Switch::NoFire);
        if k.no_fire {
            k.no_fire_since.get_or_insert(today);
        }
    }
}

/// A save from before sandbox mode that had the no-fire override on loads
/// with sandbox mode on and only that switch lifted, marked from the day the
/// override was first on.
pub(crate) fn after_load(w: &mut World) {
    let Some(k) = w.career.as_mut() else { return };
    if k.no_fire && !k.sandbox.picked {
        k.sandbox = Sandbox {
            on: true,
            since: k.no_fire_since.or(Some(w.today)),
            lifted: vec![Switch::NoFire.key().to_string()],
            picked: true,
        };
    }
}
