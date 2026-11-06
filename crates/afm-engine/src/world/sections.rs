//! The six-part day at your seat (player immersion I1,
//! `docs/design/day-sections.md`): your local clock, your sections and the
//! stops Continue makes. The world's moments stay Eastern (`calendar::zone`);
//! everything here turns them into your local time.

use crate::calendar::zone::{self, DAY};
use crate::calendar::{Date, Level, Moment, Zone};

use super::World;

/// A stop inside a section (topic 2 item 2): a live moment opens at its
/// real minute and the rest of the section plays after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LiveMoment {
    /// Day minute (after 5 AM Eastern) of world day `date`.
    pub minute: u16,
    pub date: Date,
    pub what: String,
}

impl World {
    /// The zone a team lives in, by its town.
    pub fn team_zone(&self, level: Level, t: usize) -> Zone {
        match level {
            Level::College => {
                let p = &self.programs[t];
                Zone::of_place(&self.states[p.state].code, &p.city, None)
            }
            Level::HighSchool => {
                let s = &self.high_schools[t];
                let code = &self.states[s.state].code;
                Zone::of_place(code, &s.town, super::high_school::town_place(code, &s.town))
            }
            Level::Nfl => crate::training::days::club_zone(self, t),
            Level::Other => other_zone(super::pro::other::city(t, self.year)),
            _ => Zone::Eastern,
        }
    }

    /// The zone your seat lives in (Eastern without a seat).
    pub fn seat_zone(&self) -> Zone {
        self.seat_team().map_or(Zone::Eastern, |(level, t)| self.team_zone(level, t))
    }

    /// Where the clock stands, Eastern: the last minute played of today
    /// (before 5 AM of today when it has not begun).
    pub fn now(&self) -> Moment {
        if self.played == 0 {
            zone::moment(self.today, 0).add_minutes(-1)
        } else {
            zone::moment(self.today, self.played - 1)
        }
    }

    /// An Eastern moment on your local clock.
    pub fn local(&self, at: Moment) -> Moment {
        self.seat_zone().local(at)
    }

    /// Now on your local clock.
    pub fn local_now(&self) -> Moment {
        self.local(self.now())
    }

    /// Your section now (0-5: 5 AM, 8 AM, 1 PM, 4 PM, 7 PM, 10 PM).
    pub fn section(&self) -> usize {
        Zone::section_of(self.local_now().minute)
    }

    /// "Fri 30 Oct, 7 PM": your local date and section (topic 2 item 4).
    pub fn clock_text(&self) -> String {
        let now = self.local_now();
        let start = zone::SECTION_HOURS[Zone::section_of(now.minute)] * 60;
        // Before 5 AM the night belongs to the day before's 10 PM section.
        let date = if now.minute < 5 * 60 { now.date.add(-1) } else { now.date };
        format!("{}, {}", day_text(date), zone::short_clock(start))
    }

    /// "7:30 PM": an Eastern moment on your local clock ("All day" when it
    /// has no hour).
    pub fn clock_of(&self, at: Moment) -> String {
        if !at.timed() {
            return "All day".to_string();
        }
        zone::clock(self.local(at).minute)
    }

    /// "Fri 24 Apr, 7:00 PM": an Eastern moment's local day and time.
    pub fn when_text(&self, at: Moment) -> String {
        if !at.timed() {
            return day_text(at.date);
        }
        let l = self.local(at);
        format!("{}, {}", day_text(l.date), zone::clock(l.minute))
    }

    /// Whether a moment has come on the clock (the six-part day: nothing
    /// shows before its hour). An untimed line shows from the start of its
    /// day, the day's first stop included, so a line about your own move
    /// made there (or as your career starts) is on your desk at once.
    pub fn has_come(&self, at: Moment) -> bool {
        if !at.timed() {
            return at.date <= self.today;
        }
        at <= self.now()
    }

    /// Your game kept on world day `d`, once the day has booked it.
    pub(crate) fn seat_kept_on(&self, d: Date) -> Option<&super::seat::SeatGame> {
        let (level, t) = self.seat_team()?;
        self.seat_games.iter().rev().take(3).find(|g| g.date == d && g.level == level && g.teams.contains(&t))
    }

    /// Your kickoff on world day `d`, Eastern, while your game is still to
    /// play.
    pub fn seat_kickoff(&self, d: Date) -> Option<Moment> {
        if self.seat_kept_on(d).is_some() {
            return None;
        }
        let (level, t) = self.seat_team()?;
        match level {
            Level::College => {
                let f = self
                    .college_fixtures(Some(t))
                    .into_iter()
                    .find(|f| f.date == d && f.score.is_none() && !f.cancelled)?;
                Some(Moment::at(d, f.kickoff.0, f.kickoff.1))
            }
            // The season's weeks, the playoffs and the Super Bowl.
            Level::Nfl => super::nfl_year::seat_fixture_on(self, t, d).map(|f| f.at),
            Level::HighSchool => {
                self.hs_fixtures(t).into_iter().find(|f| f.date == d)?;
                // 7 PM in the state's own zone, as the state's games are
                // played (`hs_year::state_final`).
                let code = &self.states[self.high_schools[t].state].code;
                Some(Zone::of_state(code).to_eastern(Moment::at(d, 19, 0)))
            }
            Level::Other => {
                super::pro::other::seat_fixtures(self).into_iter().find(|f| f.0 == d)?;
                Some(self.seat_zone().to_eastern(Moment::at(d, 19, 0)))
            }
            _ => None,
        }
    }

    /// Whether your game's kickoff has come today (topic 7: the game plays
    /// in its kickoff's section; the sections before it are game morning).
    pub fn kickoff_come(&self) -> bool {
        self.seat_kickoff(self.today).is_some_and(|k| k <= self.now())
    }

    /// Your game's kickoff and final whistle on world day `d`, played or
    /// not (the day's live moments).
    pub(crate) fn seat_game_window(&self, d: Date) -> Option<(Moment, Moment)> {
        let len = self.seat_game_length() as i32;
        if let Some(g) = self.seat_kept_on(d) {
            return g.whistle.map(|m| (m.add_minutes(-len), m));
        }
        self.seat_kickoff(d).map(|k| (k, k.add_minutes(len)))
    }

    /// The live moments of world day `d` at your seat (topic 2 item 2):
    /// your game's kickoff, the draft's nights, free agency's opening, the
    /// trade deadline, cutdown day and the signing days' first hour.
    pub fn live_moments(&self, d: Date) -> Vec<LiveMoment> {
        let Some(s) = self.seat else { return Vec::new() };
        let y = &self.dates;
        let mut v = Vec::new();
        let mut add = |at: Moment, what: &str| {
            let (day, m) = zone::world_day(at);
            if day == d {
                v.push(LiveMoment { minute: m, date: d, what: what.to_string() });
            }
        };
        if let Some((k, f)) = self.seat_game_window(d) {
            add(k, "Kickoff");
            add(f, "Final whistle");
        }
        match s.level {
            Level::Nfl => {
                for (k, at) in y.draft.iter().enumerate() {
                    add(*at, ["The draft: round 1", "The draft: rounds 2 and 3", "The draft: rounds 4 to 7"][k.min(2)]);
                }
                if let Some(m) = y.tampering_open {
                    add(m, "The negotiating window opens");
                }
                add(y.league_year, "The new league year: free agency opens");
                add(y.trade_deadline, "The trade deadline");
                // An hour before the deadline, so your cut plan still counts
                // (proposed, not confirmed).
                add(y.cutdown.add_minutes(-60), "Cutdown day: the 53 is due within the hour");
            }
            Level::College | Level::HighSchool => {
                let z = self.seat_zone();
                for day in [y.early_signing, Some(y.signing_day)].into_iter().flatten() {
                    add(z.to_eastern(Moment::at(day, 7, 0)), "Signing day opens");
                }
            }
            _ => {}
        }
        // Your meetings at their hours (topic 8).
        let z = self.seat_zone();
        let meetings: Vec<(Moment, String)> = self
            .folk
            .scenes
            .iter()
            .filter(|x| x.stops && !x.done && (x.date == d || x.date == d.add(1)))
            .filter_map(|x| {
                let m = x.minute?;
                Some((z.to_eastern(Moment::at(x.date, m / 60, m % 60)), format!("Meeting: {}", x.title)))
            })
            .collect();
        for (at, what) in meetings {
            add(at, &what);
        }
        v.sort_by_key(|x| x.minute);
        v
    }

    /// How long your level's game runs, minutes (the clock's final
    /// whistles).
    pub fn seat_game_length(&self) -> u16 {
        use super::clock::hour;
        match self.seat.map(|s| s.level) {
            Some(Level::College) => hour::COLLEGE_GAME,
            Some(Level::HighSchool) => hour::HS_GAME,
            _ => hour::NFL_GAME,
        }
    }

    /// Your next stop after where the clock stands: the first minute of
    /// your next section, or a live moment before it (world day, day
    /// minute). While your game is on, from kickoff to the final whistle,
    /// the sections pass: you are coaching it (topic 7).
    pub fn next_stop(&self) -> (Date, u16) {
        let z = self.seat_zone();
        for d in [self.today, self.today.add(1)] {
            let from = if d == self.today { self.played } else { 0 };
            let game = self.seat_kickoff(d).and_then(|k| {
                let (day, m) = zone::world_day(k);
                (day == d).then_some((m, m + self.seat_game_length()))
            });
            let mut cands: Vec<u16> =
                z.stops(d).into_iter().filter(|&m| game.is_none_or(|(a, b)| m <= a || m >= b)).collect();
            cands.extend(self.live_moments(d).into_iter().map(|x| x.minute));
            if let Some(m) = cands.into_iter().filter(|&m| m >= from && m < DAY).min() {
                return (d, m);
            }
        }
        (self.today.add(1), z.stops(self.today.add(1))[0])
    }

    /// Your next section's first minute only (holiday mode passes the
    /// live moments).
    pub fn next_section_stop(&self) -> (Date, u16) {
        let z = self.seat_zone();
        if let Some(m) = z.stops(self.today).into_iter().find(|&m| m >= self.played) {
            return (self.today, m);
        }
        (self.today.add(1), z.stops(self.today.add(1))[0])
    }

    /// The live moment you stand at, if the clock stopped on one.
    pub fn live_moment_now(&self) -> Option<LiveMoment> {
        let m = self.played.checked_sub(1)?;
        self.live_moments(self.today).into_iter().find(|x| x.minute == m)
    }
}

/// "Fri 24 Apr".
pub fn day_text(d: Date) -> String {
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (_, m, day) = d.parts();
    format!("{} {day} {}", d.weekday().short(), MONTHS[m as usize - 1])
}

/// A club's zone in the other pro leagues, by its town.
fn other_zone(city: &str) -> Zone {
    const PACIFIC: [&str; 12] = [
        "Seattle",
        "San Francisco",
        "Oakland",
        "Los Angeles",
        "San Diego",
        "Las Vegas",
        "Portland",
        "Sacramento",
        "San Jose",
        "Fresno",
        "BC",
        "Vancouver",
    ];
    const MOUNTAIN: [&str; 7] = ["Denver", "Salt Lake", "Calgary", "Edmonton", "Albuquerque", "Boise", "Colorado"];
    const ARIZONA: [&str; 3] = ["Phoenix", "Arizona", "Tucson"];
    const CENTRAL: [&str; 24] = [
        "Chicago",
        "Dallas",
        "Arlington",
        "Houston",
        "San Antonio",
        "Austin",
        "Memphis",
        "Nashville",
        "New Orleans",
        "St. Louis",
        "Kansas City",
        "Minnesota",
        "Milwaukee",
        "Omaha",
        "Oklahoma",
        "Tulsa",
        "Birmingham",
        "Winnipeg",
        "Saskatchewan",
        "Iowa",
        "Little Rock",
        "Green Bay",
        "Frisco",
        "Wichita",
    ];
    let has = |list: &[&str]| list.iter().any(|c| city.contains(c));
    if has(&PACIFIC) {
        Zone::Pacific
    } else if has(&ARIZONA) {
        Zone::Arizona
    } else if has(&MOUNTAIN) {
        Zone::Mountain
    } else if has(&CENTRAL) {
        Zone::Central
    } else {
        Zone::Eastern
    }
}
