//! The player page's signing line (Cain, 2026-11-15): when a player can
//! sign with your team and his first day with it, or, once he has signed,
//! the first day he is due. Every date comes from the game's own calendar
//! (`YearDates`, the portal windows by era, the CFL's season) and the rules
//! that move people: a signee joins on the January early-enrollment day or
//! at summer enrollment (`recruiting::enroll`), only FBS and FCS programs
//! sign in the early period (`signing_stage`), a junior college player
//! signs once he has a season behind him, a transfer joins with the
//! school's next block of team work (`training::rules::Frame::college`), a
//! pick joins at rookie minicamp, a veteran at the offseason program, camp
//! or the next day in season (`Frame::pro`), another league's player comes
//! back to the market when his season ends, and a high schooler changes
//! schools on transfer day (`hs_year::transfer_day`).
//!
//! The line is about your seat: a player your team can't sign at all (a
//! pro, seen from a college) gets none, and a player already with you gets
//! none once his first day has come.

use afm_engine::calendar::{Date, YearDates};
use afm_engine::world::data::Division;
use afm_engine::world::pro::{Status, cfl, leagues, money, other};
use afm_engine::world::{Person, Stage, World, hs_year};
use serde_json::{Value, json};

use crate::views::{TeamRef, date_text, mine, person_index};

/// One line: who he is to you ("Signed", "If signed", "Signed with
/// Georgia"), his first day, why that day, and when he can sign.
struct Line {
    head: String,
    first: Date,
    why: Option<&'static str>,
    or: Option<String>,
    can: Option<String>,
}

impl Line {
    fn new(head: impl Into<String>, first: Date) -> Line {
        Line { head: head.into(), first, why: None, or: None, can: None }
    }
    fn why(mut self, why: &'static str) -> Line {
        self.why = Some(why);
        self
    }
    fn or(mut self, or: Option<String>) -> Line {
        self.or = or;
        self
    }
    fn can(mut self, can: impl Into<String>) -> Line {
        self.can = Some(can.into());
        self
    }
    fn json(self) -> Value {
        let mut text = format!("{}, first day {}", self.head, date_text(self.first));
        if let Some(w) = self.why {
            text.push_str(&format!(" ({w})"));
        }
        if let Some(o) = &self.or {
            text.push_str(&format!(", or {o}"));
        }
        json!({
            "head": self.head,
            "first": date_text(self.first),
            "why": self.why,
            "or": self.or,
            "can": self.can,
            "text": text,
        })
    }
}

/// The player page's line for player `id`, if there is one for your seat.
pub fn line(w: &World, id: u32) -> Option<Value> {
    let k = person_index(w, id)?;
    let p = &w.people[k];
    // Signed with a college and not there yet: the same for every seat.
    if let Some(l) = signee(w, p) {
        return Some(l.json());
    }
    let l = match mine(w)? {
        TeamRef::College(j) => college_seat(w, p, j),
        TeamRef::Nfl(t) => nfl_seat(w, k, t),
        TeamRef::HighSchool(h) => school_seat(w, p, h),
        TeamRef::Other(fr) => other_seat(w, k, fr),
    };
    l.map(Line::json)
}

fn d(m: afm_engine::calendar::Moment) -> Date {
    m.date
}

/// The college signing days of a high school class (graduating in the
/// spring of `class_of`): the early period, signing day and January
/// enrollment in the football year before; summer enrollment in its own.
fn class_dates(class_of: i32) -> (YearDates, YearDates) {
    (YearDates::new(class_of - 1), YearDates::new(class_of))
}

/// A college signee who hasn't enrolled (or a player away on a mission or
/// in the service whose place is held): yours or another school's.
fn signee(w: &World, p: &Person) -> Option<Line> {
    let you = match crate::views::mine(w) {
        Some(TeamRef::College(j)) => Some(j),
        _ => None,
    };
    let head = |program: usize| {
        if you == Some(program) { "Signed".to_string() } else { format!("Signed with {}", w.program_name(program)) }
    };
    if let Some(s) = p.cal.signed {
        // He enrolls on the next enrollment day: January's if he chose to
        // enroll early and it is still ahead, else the summer's.
        let summer = next_summer(w);
        let first = if s.early_enrollee && w.today <= w.dates.early_enrollment && w.dates.early_enrollment < summer {
            Line::new(head(s.program), w.dates.early_enrollment).why("early enrollee")
        } else {
            Line::new(head(s.program), summer).why("summer enrollment")
        };
        return Some(first);
    }
    if p.stage == Stage::Away
        && let Some(a) = p.roads.away
        && let Some(c) = a.program
    {
        return Some(
            Line::new(head(c as usize), YearDates::new(a.until as i32).summer_enrollment)
                .why("back from his time away"),
        );
    }
    None
}

/// The next summer enrollment day on or after today.
fn next_summer(w: &World) -> Date {
    let here = w.dates.summer_enrollment;
    if w.today <= here { here } else { YearDates::new(w.year + 1).summer_enrollment }
}

/// A college seat (program `j`).
fn college_seat(w: &World, p: &Person, j: usize) -> Option<Line> {
    let division = w.programs[j].division;
    // Only FBS and FCS programs sign in the early period.
    let early_ok = matches!(division, Division::FBS | Division::FCS);
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool => recruit(w, p, early_ok && p.stage == Stage::HighSchool, "If signed"),
        Stage::College => {
            let c = p.college?;
            if c == j && p.cut_here().is_none() {
                return None;
            }
            if w.programs[c].division == Division::JUCO && p.cal.portal.is_none() && p.cut_here().is_none() {
                return juco(w, p, early_ok);
            }
            transfer(w, p)
        }
        _ => None,
    }
}

/// A high school recruit for a college: when he can sign, and his first
/// day on campus (the class's January enrollment if he signs early and
/// enrolls early, else its summer enrollment).
fn recruit(w: &World, p: &Person, early_ok: bool, head: &str) -> Option<Line> {
    let (winter, summer) = class_dates(p.class_of.max(w.year));
    let today = w.today;
    // The class's last chance is summer enrollment: after it he is a
    // year late, and the next class's days apply.
    if today > summer.summer_enrollment {
        let (winter, summer) = class_dates(w.year + 1);
        return Some(
            Line::new(head, summer.summer_enrollment)
                .why("summer enrollment")
                .can(format!("can sign from {}", date_text(winter.signing_day))),
        );
    }
    let june = Line::new(head, summer.summer_enrollment).why("summer enrollment");
    match winter.early_signing.filter(|&e| early_ok && today <= e.add(2)) {
        Some(e) => {
            let can = if today >= e {
                "can sign now, in the early period".into()
            } else {
                format!("can sign from {}", date_text(e))
            };
            Some(june.or(Some(format!("{} if he enrolls early", date_text(winter.early_enrollment)))).can(can))
        }
        // Before the early period (to 2016), early enrollees signed in
        // January and enrolled at once (`SignStage::JanuaryEnrollees`).
        None if winter.early_signing.is_none() && early_ok && today <= winter.early_enrollment => Some(
            june.or(Some(format!("{} if he enrolls early", date_text(winter.early_enrollment))))
                .can(format!("can sign from {}", date_text(winter.early_enrollment))),
        ),
        None if today < winter.signing_day => {
            Some(june.can(format!("can sign from {}", date_text(winter.signing_day))))
        }
        None => Some(june.can(format!("can sign now, until {}", date_text(summer.summer_enrollment)))),
    }
}

/// A junior college player for a four-year college: he can sign once he
/// has a season behind him (or is in it), in the early period if your
/// program has one (he enrolls in January) or from signing day (summer).
fn juco(w: &World, p: &Person, early_ok: bool) -> Option<Line> {
    let today = w.today;
    // Still unsigned after signing day: the late days run to summer
    // enrollment (`late_signing_day`).
    if p.cal.late && today <= w.dates.summer_enrollment {
        let june = w.dates.summer_enrollment;
        return Some(
            Line::new("If signed", june)
                .why("summer enrollment")
                .can(format!("can sign now, until {}", date_text(june))),
        );
    }
    // He signs in this football year's winter: by December he has a
    // season behind him (`signing_stage`, `juco`).
    let y = w.dates.clone();
    let summer = |y: &YearDates| YearDates::new(y.season + 1).summer_enrollment;
    let june = Line::new("If signed", summer(&y)).why("summer enrollment");
    match y.early_signing.filter(|&e| early_ok && today <= e.add(2)) {
        Some(e) => {
            let can = if today >= e {
                "can sign now, in the early period".into()
            } else {
                format!("can sign from {}", date_text(e))
            };
            Some(
                Line::new("If signed", y.early_enrollment)
                    .why("early enrollee")
                    .or(Some(format!("{} if he signs in February", date_text(summer(&y)))))
                    .can(can),
            )
        }
        None if today < y.signing_day => Some(june.can(format!("can sign from {}", date_text(y.signing_day)))),
        None => Some(june.can(format!("can sign now, until {}", date_text(summer(&y))))),
    }
}

/// A four-year college player for another college: only through the
/// portal. In it, he decides on his day; out of it, the next window.
fn transfer(w: &World, p: &Person) -> Option<Line> {
    let today = w.today;
    if let Some(e) = p.cal.portal {
        let day = if e.on_hold() { None } else { Some(e.decide_by.max(today)) };
        let sign = day.unwrap_or(today);
        let (first, why) = college_first_day(sign);
        let can = match day {
            Some(dd) if dd > today => format!("in the portal, decides by {}", date_text(dd)),
            Some(_) => "in the portal, can sign now".to_string(),
            None => "in the portal once his team's playoff run ends".to_string(),
        };
        return Some(Line::new("If signed", first).why(why).can(can));
    }
    let (open, close) = next_window(w)?;
    let sign = open.max(today);
    let (first, why) = college_first_day(sign);
    let can = if open <= today {
        format!("can sign if he enters the portal, open until {}", date_text(close))
    } else {
        format!("can sign if he enters the portal, next window {} to {}", date_text(open), date_text(close))
    };
    Some(Line::new("If signed", first).why(why).can(can))
}

/// The next regular portal window still open or ahead.
fn next_window(w: &World) -> Option<(Date, Date)> {
    let mut v: Vec<(Date, Date)> =
        (w.year - 1..=w.year + 1).flat_map(|s| afm_engine::world::portal_windows_for(w, s)).collect();
    v.sort();
    v.into_iter().find(|x| x.1 >= w.today)
}

/// A transfer's first day with a college that signs him on `sign`: the
/// spring semester when he signs before it (or in its first two weeks, the
/// day after); summer enrollment when he signs in the spring; the day
/// after in summer and the season; the next spring semester in December.
fn college_first_day(sign: Date) -> (Date, &'static str) {
    let jan = YearDates::new(sign.year() - 1).spring_semester;
    let summer = YearDates::new(sign.year()).summer_enrollment;
    if sign < jan {
        (jan, "spring semester")
    } else if sign <= jan.add(14) {
        (sign.add(1), "the day after he signs")
    } else if sign < summer {
        (summer, "summer enrollment")
    } else if sign.month() == 12 {
        (YearDates::new(sign.year()).spring_semester, "spring semester")
    } else {
        (sign.add(1), "the day after he signs")
    }
}

/// An NFL seat (club `t`).
fn nfl_seat(w: &World, k: usize, t: usize) -> Option<Line> {
    let p = &w.people[k];
    let today = w.today;
    match p.stage {
        Stage::HighSchool | Stage::PrepSchool | Stage::College => {
            // Three years out of high school, and declared by January's
            // deadline (the draft after it otherwise).
            let next = if today <= w.dates.declare_deadline { w.year + 1 } else { w.year + 2 };
            let year = next.max(p.class_of + 3);
            let y = YearDates::new(year);
            let junior = p.college.is_some_and(|c| w.programs[c].division == Division::JUCO);
            let can = if junior {
                format!("draft eligible once he moves to a four-year school; {year} draft at the earliest")
            } else {
                format!("can be drafted in the {year} draft at the earliest, {}", date_text(d(y.draft[0])))
            };
            Some(Line::new("If drafted", y.rookie_minicamp).why("rookie minicamp").can(can))
        }
        Stage::Declared => {
            let y = if today <= d(w.dates.draft[2]) { w.dates.clone() } else { YearDates::new(w.year + 1) };
            Some(Line::new("If drafted", y.rookie_minicamp).why("rookie minicamp").can(format!(
                "in the {} draft, {}; undrafted, can sign from {}",
                y.season,
                date_text(d(y.draft[0])),
                date_text(d(y.udfa_rush))
            )))
        }
        Stage::Pro => pro_for_nfl(w, k, t),
        _ => None,
    }
}

/// A pro seen from NFL club `t`.
fn pro_for_nfl(w: &World, k: usize, t: usize) -> Option<Line> {
    let p = &w.people[k];
    let c = p.pro.as_ref()?;
    let today = w.today;
    let rookie = c.accrued == 0 && c.first_year >= w.year;
    if c.team == Some(t) {
        // Yours: only while he is new and his first day is still ahead. A
        // reserve/future deal starts with next year's offseason program.
        if !c.new_team {
            return None;
        }
        let future = c.contract.as_ref().is_some_and(|x| x.kind == money::Kind::Future && x.signed > w.year);
        if future {
            let first = YearDates::new(w.year + 1).offseason_returning;
            return Some(Line::new("Signed", first).why("offseason program"));
        }
        let (first, why) = nfl_first_day(today, rookie);
        let due = if rookie { today <= w.dates.rookie_minicamp } else { today < w.dates.offseason_returning };
        return (first > today && due).then(|| Line::new("Signed", first).why(why));
    }
    match c.status {
        Status::FreeAgent => {
            let (first, why) = nfl_first_day(today, rookie);
            let can = if c.on_waivers { "on waivers, can be claimed" } else { "free agent, can sign now" };
            Some(Line::new("If signed", first).why(why).can(can))
        }
        Status::Other(lg) => {
            let back = other_back(lg, today)?;
            let (first, why) = nfl_first_day(back, rookie);
            Some(Line::new("If signed", first).why(why).can(format!(
                "playing in the {}, back on the market {}",
                other::league_short(lg),
                date_text(back)
            )))
        }
        Status::PracticeSquad => {
            let (first, why) = nfl_first_day(today, rookie);
            Some(
                Line::new("If signed", first)
                    .why(why)
                    .can("on another club's practice squad, can sign to your roster now"),
            )
        }
        Status::Roster | Status::Reserve => {
            let club = c.team.map(|x| w.nfl_team_name(x)).unwrap_or_default();
            // A trade: open from the new league year to the deadline.
            let (open, at, ly) = if today < d(w.dates.league_year) {
                (false, d(w.dates.league_year), w.year)
            } else if today <= d(w.dates.trade_deadline) {
                (true, today, w.year)
            } else {
                (false, d(YearDates::new(w.year + 1).league_year), w.year + 1)
            };
            // A contract that ends before the next league year: a free agent then.
            let ends = c.contract.as_ref().map(|x| x.last_year());
            if !open && ends.is_some_and(|e| e < ly) {
                let (first, why) = nfl_first_day(at, false);
                return Some(
                    Line::new("If signed", first)
                        .why(why)
                        .can(format!("free agent from {} unless they keep him", date_text(at))),
                );
            }
            let (first, why) = nfl_first_day(at, rookie && open);
            let can = if open {
                format!("under contract with the {club}, trades open until {}", date_text(d(w.dates.trade_deadline)))
            } else {
                format!("under contract with the {club}, trades open again {}", date_text(at))
            };
            Some(Line::new("If traded", first).why(why).can(can))
        }
    }
}

/// An NFL newcomer's first day when he joins on `day` (the club's year as
/// the game runs it, `Frame::pro`): the offseason program before it opens
/// (a rookie, rookie minicamp before it); during the program and the
/// season the next day; in the summer break the day his group reports to
/// camp; after Black Monday the next offseason program.
fn nfl_first_day(day: Date, rookie: bool) -> (Date, &'static str) {
    let y = year_of(day);
    if rookie && day <= y.rookie_minicamp {
        return (y.rookie_minicamp, "rookie minicamp");
    }
    if day < y.offseason_returning {
        return (y.offseason_returning, "offseason program");
    }
    if day <= y.minicamp.1 {
        return (day.add(1), "the day after he signs");
    }
    let camp = if rookie { y.rookies_report } else { y.veterans_report };
    if day < camp {
        return (camp, if rookie { "rookies report" } else { "veterans report" });
    }
    if day < y.black_monday {
        return (day.add(1), "the day after he signs");
    }
    (YearDates::new(y.season + 1).offseason_returning, "next offseason program")
}

/// The football year a date falls in.
fn year_of(day: Date) -> YearDates {
    let y = YearDates::new(day.year());
    if day < y.start { YearDates::new(day.year() - 1) } else { y }
}

/// When a player in another league comes back to the market: the week
/// after its title game (a league with its teams in the world) or the end
/// of its last month.
fn other_back(lg: u8, today: Date) -> Option<Date> {
    for year in [today.year(), today.year() + 1] {
        let end = if other::full(lg, year) {
            other::season_span(lg, year).map(|s| s.2)
        } else {
            let l =
                if (lg as usize) < leagues::LEAGUES.len() { leagues::LEAGUES[lg as usize] } else { leagues::USFL_2022 };
            (year >= l.from && year <= l.to).then(|| Date::ymd(year, l.ends, 28))
        };
        if let Some(e) = end.filter(|&e| e >= today) {
            return Some(e);
        }
    }
    None
}

/// A high school seat (school `h`): your players' college signing days,
/// and another school's player's transfer day.
fn school_seat(w: &World, p: &Person, h: usize) -> Option<Line> {
    if !matches!(p.stage, Stage::HighSchool) {
        return None;
    }
    if p.hs == h {
        // Your player: when he can sign with a college and his first day there.
        return recruit(w, p, true, "If he signs with a college");
    }
    let mut day = hs_year::transfer_day(w.year);
    if day < w.today {
        day = hs_year::transfer_day(w.year + 1);
    }
    Some(Line::new("If he transfers here", day).why("transfer day").can("sits out under your state's rule"))
}

/// Another league's seat (franchise `fr`): its rosters fill when camp
/// opens, from the street and the drafts.
fn other_seat(w: &World, k: usize, fr: usize) -> Option<Line> {
    let p = &w.people[k];
    let lg = other::league_of(fr);
    let today = w.today;
    // Camp opens three weeks before the opener (`other::day`).
    let camp = |year: i32| other::format(lg, year).filter(|_| other::full(lg, year)).map(|f| f.start.add(-21));
    let season = |year: i32| other::season_span(lg, year).filter(|_| other::full(lg, year));
    // The first day for a player who joins on `day`: camp before it, the
    // next day in season, next year's camp after it.
    let first = |day: Date| -> Option<(Date, &'static str)> {
        for year in [day.year(), day.year() + 1] {
            let (Some(c), Some(s)) = (camp(year), season(year)) else { continue };
            if day < c {
                return Some((c, "camp opens"));
            }
            if day < s.2 {
                return Some((day.add(1), "the day after he signs"));
            }
        }
        None
    };
    match p.stage {
        Stage::CflPool if lg == other::CFL => {
            if let Some(&(holder, _)) = w.others.rights.get(&p.player.id) {
                // Back at school for the fall: the camp after it.
                let after = if today.month() <= 4 { today } else { Date::ymd(today.year() + 1, 1, 1) };
                let (c, why) = first(after)?;
                let head = if holder as usize == fr {
                    "Your pick".to_string()
                } else {
                    format!("{} hold his rights", other::name(holder as usize, w.year))
                };
                return Some(Line::new(head, c).why(why).can("back at school for his last year"));
            }
            let year = cfl::coming_draft(w);
            let day = cfl::draft_date(year);
            let (c, why) = first(day)?;
            Some(Line::new("If drafted", c).why(why).can(format!("in the {year} CFL draft, {}", date_text(day))))
        }
        Stage::Pro => {
            let c = p.pro.as_ref()?;
            let here = w.others.team_of(k);
            if here == Some(fr) {
                return None;
            }
            match c.status {
                Status::FreeAgent => {
                    let (f, why) = first(today)?;
                    Some(Line::new("If signed", f).why(why).can("free agent, can sign now"))
                }
                Status::Other(l) if l != lg => {
                    let back = other_back(l, today)?;
                    let (f, why) = first(back)?;
                    Some(Line::new("If signed", f).why(why).can(format!(
                        "playing in the {}, back on the market {}",
                        other::league_short(l),
                        date_text(back)
                    )))
                }
                Status::Other(_) => {
                    let back = other_back(lg, today)?;
                    let (f, why) = first(back.add(1))?;
                    let club = here.map(|x| other::name(x, w.year).to_string()).unwrap_or_default();
                    Some(
                        Line::new("If signed", f)
                            .why(why)
                            .can(format!("with {club}, free when the season ends {}", date_text(back))),
                    )
                }
                // Under an NFL contract: he comes to you only if released.
                _ => None,
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transfer_joins_with_the_next_block_of_team_work() {
        // The January window (Jan 4-13, 2027): the spring semester.
        let (first, why) = college_first_day(Date::ymd(2027, 1, 5));
        assert_eq!((first, why), (YearDates::new(2026).spring_semester, "spring semester"));
        // A December window (2025): the spring semester after it.
        assert_eq!(college_first_day(Date::ymd(2025, 12, 15)).0, Date::ymd(2026, 1, 12));
        // The spring window: summer enrollment.
        assert_eq!(college_first_day(Date::ymd(2025, 4, 20)).0, YearDates::new(2025).summer_enrollment);
        // Fall camp: the next day.
        assert_eq!(college_first_day(Date::ymd(2026, 8, 3)).0, Date::ymd(2026, 8, 4));
    }

    #[test]
    fn an_nfl_newcomer_starts_on_the_clubs_calendar() {
        let y = YearDates::new(2026);
        // A pick: rookie minicamp.
        assert_eq!(nfl_first_day(y.draft[0].date, true), (y.rookie_minicamp, "rookie minicamp"));
        // Free agency in March: the offseason program.
        assert_eq!(nfl_first_day(y.league_year.date, false), (y.offseason_returning, "offseason program"));
        // The summer break: the day veterans report.
        assert_eq!(nfl_first_day(y.minicamp.1.add(3), false).0, y.veterans_report);
        // In season: the next day.
        let oct = Date::ymd(2026, 10, 14);
        assert_eq!(nfl_first_day(oct, false).0, oct.add(1));
        // After Black Monday: next year's offseason program.
        assert_eq!(nfl_first_day(y.black_monday.add(2), false).0, YearDates::new(2027).offseason_returning);
    }

    #[test]
    fn a_class_signs_in_the_winter_before_it_graduates() {
        let (winter, summer) = class_dates(2027);
        assert_eq!(winter.early_signing, Some(Date::ymd(2026, 12, 2)));
        assert_eq!(winter.signing_day.year(), 2027);
        assert_eq!(winter.early_enrollment, Date::ymd(2027, 1, 10));
        assert_eq!(summer.summer_enrollment.year(), 2027);
    }
}
