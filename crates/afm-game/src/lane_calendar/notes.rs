//! Roster notes on the Calendar (playtest bug 8, minor bug 381): a short
//! plain sentence on the dates that move players on or off a roster, or
//! change a class or an eligibility year. Cain marked the list in the
//! thread "Explain rule dates on the calendar".
//!
//! Nothing here is stored and no sentence is typed against a date. A note
//! is found by the football year's own dates (`YearDates`), and the numbers
//! in it come from that year's rule book (`RuleBook`) or from the game's own
//! rule functions, so a roster-size vote, a new era or a moved date changes
//! the note with it. The three dates the clock typed in itself are now
//! shared functions both read (`anchors::eligibility_day`,
//! `hs_year::class_day`, `hs_year::transfer_day`, and
//! `market::VESTED_SEASONS` for the waiver rule).
//!
//! A note either rides under a line the Calendar already has on that date
//! (a sentence in the day panel only, `title` None) or is a line of its own
//! where the date had none (`title` Some: it shows in the month cell too).
//! The month cells and the Ahead list are otherwise unchanged.

use afm_engine::calendar::anchors::eligibility_day;
use afm_engine::calendar::{Date, Moment, RuleBook, YearDates};
use afm_engine::world::data::Division;
use afm_engine::world::eligibility as el;
use afm_engine::world::hs_year;
use afm_engine::world::pro::market::VESTED_SEASONS;

use super::{K, Lv, short_day};

/// One note on one day.
pub(super) struct Hit {
    pub date: Date,
    /// The line's own title when the Calendar had no line on this date, and
    /// its kind; None when a line is there and the note rides under it.
    pub line: Option<(&'static str, K)>,
    pub text: String,
    /// The Eastern hour of a date that has one (the new league year, the
    /// draft nights), so the note sorts beside its line.
    pub at: Option<Moment>,
}

impl Hit {
    fn under(date: Date, text: impl Into<String>) -> Hit {
        Hit { date, line: None, text: text.into(), at: None }
    }
    fn own(date: Date, title: &'static str, k: K, text: impl Into<String>) -> Hit {
        Hit { date, line: Some((title, k)), text: text.into(), at: None }
    }
    fn at(mut self, m: Moment) -> Hit {
        self.at = Some(m);
        self
    }
}

/// What a high school seat needs of its state and the world.
pub(super) struct Hs {
    /// The first game of your state's season.
    pub first_game: Date,
    /// The day the new class joins the rosters (the earliest state decides).
    pub class_day: Date,
}

/// Every note of a football year at a level, with its date.
pub(super) fn all(y: &YearDates, rules: &RuleBook, lv: Lv, hs: Option<&Hs>) -> Vec<Hit> {
    let mut out = Vec::new();
    match lv {
        Lv::Nfl => nfl(y, rules, &mut out),
        Lv::College => college(y, rules, &mut out),
        Lv::High => high(y, hs, &mut out),
        Lv::Other => {}
    }
    out
}

fn nfl(y: &YearDates, rules: &RuleBook, out: &mut Vec<Hit>) {
    let (offseason, roster) = (rules.nfl_offseason, rules.nfl_roster);
    out.push(
        Hit::under(
            y.tag_deadline.date,
            "Last day to tag or tender your expiring players. The ones you don't leave when the new league year opens.",
        )
        .at(y.tag_deadline),
    );
    out.push(
        Hit::under(
            y.league_year.date,
            format!(
                "Contracts that end this year end at {} and those players leave rosters. Tagged players stay. \
                 Rosters may hold {offseason} until the cutdown.",
                y.league_year.clock()
            ),
        )
        .at(y.league_year),
    );
    for night in y.draft {
        out.push(Hit::under(night.date, "Each pick joins his club's roster at once on his rookie deal.").at(night));
    }
    out.push(
        Hit::own(
            y.udfa_rush.date,
            "Undrafted free agents sign",
            K::Rec,
            format!("Undrafted players sign within hours of the last pick, up to each club's {offseason}."),
        )
        .at(y.udfa_rush),
    );
    out.push(Hit::under(y.veterans_report, "Every club is topped up to full depth with minimum-salary signings."));
    out.push(
        Hit::under(
            y.cutdown.date,
            format!(
                "Rosters go from {offseason} to {roster} at the cutdown. Young players near the line move to the \
                 practice squad and the rest are released."
            ),
        )
        .at(y.cutdown),
    );
    out.push(
        Hit::under(
            y.waiver_claims.date,
            format!(
                "Players cut with under {} seasons can be claimed by other clubs on the next waiver run. \
                 Unclaimed, they become free agents.",
                number_word(VESTED_SEASONS)
            ),
        )
        .at(y.waiver_claims),
    );
    out.push(Hit::under(
        y.trade_deadline.date,
        "Last day for trades. There are none from tomorrow until the new league year.",
    ));
    out.push(Hit::own(
        y.trade_deadline.date.add(1),
        "Every release goes on waivers",
        K::World,
        format!(
            "From today every released player goes on waivers, not only those under {} seasons, until Black Monday.",
            number_word(VESTED_SEASONS)
        ),
    ));
    out.push(Hit::under(
        y.black_monday,
        "Clubs that missed the playoffs give the practice-squad players they want a minimum deal for next year \
         and release the rest.",
    ));
    out.push(Hit::under(
        y.super_bowl,
        "Players decide whether to retire, and retirees leave rosters at once. Playoff clubs' practice-squad \
         players get next-year deals or go.",
    ));
}

fn college(y: &YearDates, rules: &RuleBook, out: &mut Vec<Hit>) {
    if let Some(d) = y.early_signing {
        out.push(Hit::under(
            d,
            "Recruits who sign now count against your roster limit but join the roster only when they enroll.",
        ));
    }
    out.push(Hit::own(eligibility_day(y.season), "Class years move up", K::World, eligibility_text(rules, y.season)));
    out.push(Hit::under(
        y.portal_open,
        "Players you cut since the last window enter the portal today. Others enter on their own days until it closes.",
    ));
    out.push(Hit::under(y.declare_deadline, declare_text(y)));
    // Teams still in the playoffs come to the same two days later.
    let late = y.title_game.add(4);
    out.push(Hit::own(
        y.declare_withdraw,
        "Draft withdrawals",
        K::World,
        "Declared players who pull out return to the roster. Every other declared player leaves your roster list today.",
    ));
    if late > y.declare_deadline {
        out.push(Hit::own(
            y.title_game.add(7),
            "Draft withdrawals",
            K::World,
            "Declared players on teams that played in the playoffs: those who pull out return to the roster, \
             and every other declared player leaves your roster list today.",
        ));
    }
    out.push(Hit::own(
        y.early_enrollment,
        "Early enrollees arrive",
        K::Rec,
        "Signees who chose to enroll early join the roster today for spring practice.",
    ));
    out.push(Hit::under(
        y.signing_day,
        "Committed recruits sign and join the roster when they enroll. Recruits nobody signs keep looking until \
         June or leave football.",
    ));
    out.push(Hit::own(
        y.summer_enrollment,
        "Summer enrollment",
        K::Rec,
        "All remaining signees join the roster today. Players leaving for a mission or service come off it.",
    ));
    out.push(Hit::under(
        y.fall_camp,
        "Portal entrants still without a school decide today. Walk-ons in the two-deep can earn a scholarship \
         where a place is free.",
    ));
}

fn high(y: &YearDates, hs: Option<&Hs>, out: &mut Vec<Hit>) {
    out.push(Hit::under(y.hs_school_start, "Every player's grade moves up a year today. Nobody joins or leaves."));
    if let Some(hs) = hs {
        out.push(Hit::own(
            hs.class_day,
            "New freshman class",
            K::World,
            "The new freshman class joins school rosters.",
        ));
        out.push(Hit::own(
            hs.first_game.add(-1),
            "Players age a year",
            K::World,
            "Players age a year and take the yearly growth step. Players in trouble are suspended or dismissed for \
             the season, and a few quit football.",
        ));
    }
    out.push(Hit::own(
        hs_year::transfer_day(y.season),
        "Transfer day",
        K::World,
        "Players can transfer between schools today. Each sits out under your state's rule. A few 16-year-olds \
         are moved up or held back a class.",
    ));
    if let Some(d) = y.early_signing {
        out.push(Hit::under(d, "Seniors who commit sign now. They stay on your roster until they enroll."));
    }
    out.push(Hit::under(
        y.signing_day,
        "Seniors who signed stay on your roster until they enroll. Seniors nobody signs and no college is still \
         looking at leave football today.",
    ));
    out.push(Hit::own(
        y.early_enrollment,
        "Early enrollees leave",
        K::Rec,
        "Signed seniors who chose to enroll early leave your roster today.",
    ));
    out.push(Hit::own(
        y.summer_enrollment,
        "Seniors enroll",
        K::Rec,
        "Every other signed senior leaves your roster today.",
    ));
}

/// The eligibility-day sentence, with the rule of the season just played.
fn eligibility_text(rules: &RuleBook, season: i32) -> String {
    let counted = "Each player's season on the roster is counted and his class moves up a year.";
    if el::five_in_five(season, Division::FBS) {
        let n = number_word(el::seasons_allowed(season, Division::FBS));
        format!("{counted} FBS and FCS: {n} seasons in {n} years, so no redshirt keeps a year.")
    } else if rules.four_game_redshirt {
        format!("{counted} A redshirt who plays four games or fewer keeps his year.")
    } else {
        format!("{counted} Any game a redshirt plays uses up his year.")
    }
}

/// The declaration-deadline sentence: the playoff teams' later day is named
/// only in a year when the title game falls after the deadline.
fn declare_text(y: &YearDates) -> String {
    let late = y.title_game.add(4);
    let base = "Players with no eligibility left, and underclassmen going pro, declare today.";
    if late > y.declare_deadline {
        format!("{base} Teams still in the playoffs wait until {}.", short_day(late))
    } else {
        base.to_string()
    }
}

fn number_word(n: u32) -> &'static str {
    match n {
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        _ => "several",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use afm_engine::calendar::anchors::january_window;

    fn on(hits: &[Hit], d: Date) -> Vec<&Hit> {
        hits.iter().filter(|h| h.date == d).collect()
    }

    #[test]
    fn nfl_notes_read_the_rule_book_and_the_dates() {
        let y = YearDates::new(2026);
        let rules = RuleBook::era(2026);
        let hits = all(&y, &rules, Lv::Nfl, None);
        let cut = on(&hits, y.cutdown.date);
        assert_eq!(cut.len(), 1);
        assert!(
            cut[0].text.contains(&format!("from {} to {}", rules.nfl_offseason, rules.nfl_roster)),
            "{}",
            cut[0].text
        );
        assert_eq!(cut[0].at, Some(y.cutdown));
        // A roster-size vote moves the number with it.
        let mut bigger = rules.clone();
        bigger.nfl_roster += 1;
        let hits = all(&y, &bigger, Lv::Nfl, None);
        assert!(on(&hits, y.cutdown.date)[0].text.contains(&format!("to {}", rules.nfl_roster + 1)));
        // The league year's own hour comes from the date, and the offseason limit from the rules.
        let ly = on(&hits, y.league_year.date);
        assert!(ly[0].text.contains(&y.league_year.clock()), "{}", ly[0].text);
        assert!(ly[0].text.contains(&rules.nfl_offseason.to_string()));
        // All three draft nights, the day after the deadline as its own line.
        for night in y.draft {
            assert!(on(&hits, night.date).iter().any(|h| h.text.contains("rookie deal")));
        }
        let after = on(&hits, y.trade_deadline.date.add(1));
        assert!(after[0].line.is_some_and(|l| l.0 == "Every release goes on waivers"));
        assert!(after[0].text.contains("four seasons"), "{}", after[0].text);
        // No other level's note.
        assert!(!hits.iter().any(|h| h.text.contains("class moves up")));
    }

    #[test]
    fn college_notes_follow_the_eligibility_day_and_the_era() {
        let y = YearDates::new(2026);
        let rules = RuleBook::era(2026);
        let hits = all(&y, &rules, Lv::College, None);
        let day = on(&hits, eligibility_day(2026));
        assert_eq!(day.len(), 1);
        assert_eq!(day[0].line.map(|l| l.0), Some("Class years move up"));
        assert!(day[0].text.contains("five seasons in five years"), "{}", day[0].text);
        // The same day the clock settles eligibility, before the window opens.
        assert_eq!(eligibility_day(2026), Date::ymd(2027, 1, 2));
        assert!(eligibility_day(2026) <= january_window(2026).0);
        // The four-game redshirt era and the old era read their own rule.
        let y22 = YearDates::new(2022);
        let r22 = RuleBook::era(2022);
        let h22 = all(&y22, &r22, Lv::College, None);
        let t22 = &on(&h22, eligibility_day(2022))[0].text;
        assert!(t22.contains("four games or fewer"), "{t22}");
        let y16 = YearDates::new(2016);
        let r16 = RuleBook::era(2016);
        let h16 = all(&y16, &r16, Lv::College, None);
        assert!(on(&h16, eligibility_day(2016))[0].text.contains("uses up his year"));
        // Signing day, portal day and the draft days carry their notes.
        assert!(on(&hits, y.signing_day).iter().any(|h| h.text.contains("Committed recruits sign")));
        assert!(on(&hits, y.portal_open).iter().any(|h| h.text.contains("you cut")));
        assert!(on(&hits, y.declare_withdraw).iter().any(|h| h.text.contains("leaves your roster list")));
    }

    #[test]
    fn the_declaration_deadline_names_the_playoff_day_only_when_it_is_later() {
        for s in 2016..2032 {
            let y = YearDates::new(s);
            let t = declare_text(&y);
            let later = y.title_game.add(4) > y.declare_deadline;
            assert_eq!(t.contains("playoffs wait until"), later, "{s}: {t}");
        }
    }

    #[test]
    fn high_school_notes_need_the_state_and_other_levels_get_none() {
        let y = YearDates::new(2026);
        let rules = RuleBook::era(2026);
        let hs = Hs {
            first_game: Date::ymd(2026, 8, 28),
            class_day: hs_year::class_day(y.hs_school_start, Date::ymd(2026, 8, 20)),
        };
        let hits = all(&y, &rules, Lv::High, Some(&hs));
        assert_eq!(hs.class_day, Date::ymd(2026, 8, 10).min(y.hs_school_start));
        assert!(on(&hits, Date::ymd(2026, 8, 27))[0].text.contains("age a year"));
        assert!(on(&hits, hs_year::transfer_day(2026))[0].text.contains("transfer between schools"));
        assert!(on(&hits, y.hs_school_start).iter().any(|h| h.text.contains("grade moves up")));
        // Without a state there is no season-eve or class-day note, the rest stay.
        let bare = all(&y, &rules, Lv::High, None);
        assert!(bare.len() < hits.len());
        assert!(all(&y, &rules, Lv::Other, None).is_empty());
    }
}
