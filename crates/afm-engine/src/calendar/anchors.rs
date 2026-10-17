//! The real dates of one football year, worked out from real anchors (topic
//! 1.3): Labor Day, Thanksgiving, the Super Bowl Sunday and the rest. A
//! football year `season` runs from the day after one Super Bowl (February of
//! `season`) to the next Super Bowl (February of `season + 1`), so the fall
//! season `season` sits inside it.
//!
//! 2026 uses the real 2026 dates where they are published (NFL.com "2026
//! National Football League important dates"; the CFP's 2026-27 schedule,
//! announced June 1, 2026). Other years shift the way the real anchors do.
//! Era differences (the Saturday draft before 2010, the BCS title game, the
//! 4-team playoff) come from `rules.rs`.

use super::date::{Date, Moment, Weekday};

/// When a year's college postseason format is used (topic 11).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CfbPostseason {
    /// Bowls only; the champion is the final AP poll's No. 1 (before 1998).
    PollsOnly,
    /// The BCS title game between the top two (1998-2013).
    Bcs,
    /// The 4-team College Football Playoff (2014-2023).
    FourTeam,
    /// The 12-team playoff (2024 on).
    TwelveTeam,
}

/// Every anchor date of one football year.
#[derive(Clone, Debug)]
pub struct YearDates {
    pub season: i32,
    /// The day after the previous Super Bowl: the year's first day.
    pub start: Date,
    /// The Super Bowl that ends the year (February of `season + 1`).
    pub super_bowl: Date,

    // --- NFL offseason (league year `season`) ---
    /// Last season's Pro Bowl when it was played the Sunday after its Super
    /// Bowl (seasons to 2008), which puts it inside this football year.
    pub pro_bowl_prev: Option<Date>,
    pub tag_window_open: Date,
    pub tag_deadline: Moment,
    pub combine_start: Date,
    pub combine_end: Date,
    pub combine_invites: Date,
    /// The negotiating window before free agency (2013 on).
    pub tampering_open: Option<Moment>,
    pub league_year: Moment,
    pub roster_bonus_window: (Date, Date),
    pub annual_meeting: (Date, Date),
    pub comp_picks: Date,
    pub pro_days: (Date, Date),
    pub offseason_new_coach: Date,
    pub offseason_returning: Date,
    pub top30_deadline: Date,
    pub rfa_deadline: Date,
    pub rfa_match_deadline: Date,
    /// Round 1 night, rounds 2-3, rounds 4-7.
    pub draft: [Moment; 3],
    pub udfa_rush: Moment,
    pub fifth_year_option: Date,
    pub rookie_minicamp: Date,
    pub rookie_program: Date,
    pub rookie_premiere: (Date, Date),
    pub schedule_release: Date,
    pub spring_meeting: Date,
    pub otas: (Date, Date),
    pub june_1: Date,
    pub minicamp: (Date, Date),
    pub ring_ceremony: Date,
    pub rookie_readiness: Date,
    pub summer_break: (Date, Date),
    pub tag_long_term_deadline: Moment,
    pub rookies_report: Date,
    pub veterans_report: Date,
    pub hof_game: Date,
    pub hof_enshrinement: Date,
    pub preseason: [Date; 3],
    pub cutdown: Moment,
    pub waiver_claims: Moment,
    pub practice_squads: Date,

    // --- NFL season ---
    pub nfl_kickoff: Date,
    /// The Sunday of each regular-season week.
    pub nfl_weeks: Vec<Date>,
    pub thanksgiving: Date,
    pub christmas: Date,
    pub trade_deadline: Moment,
    pub pro_bowl_voting: (Date, Date),
    pub black_monday: Date,
    pub wild_card: Date,
    pub divisional: Date,
    pub conference_title_games: Date,
    pub nfl_honors: Date,
    pub all_pro: Date,
    pub hof_finalists: Date,
    /// The Pro Bowl of this season (None: the 2020 season's was not played).
    pub pro_bowl_games: Option<Date>,

    // --- College ---
    pub labor_day: Date,
    pub coaches_preseason_poll: Date,
    pub ap_preseason_poll: Date,
    pub media_days: (Date, Date),
    pub fall_camp: Date,
    /// Saturdays from Week 0 to rivalry week.
    pub cfb_weeks: Vec<Date>,
    pub rivalry_week: Date,
    pub conference_championships: Date,
    pub army_navy: Date,
    pub selection_day: Date,
    pub first_cfp_ranking: Date,
    pub heisman_finalists: Date,
    pub awards_show: Date,
    pub heisman: Date,
    pub all_america: Date,
    pub finals_week: (Date, Date),
    pub december_graduation: Date,
    pub bowl_season: (Date, Date),
    pub cfp_first_round: (Date, Date),
    pub cfp_quarterfinals: (Date, Date),
    pub cfp_semifinals: (Date, Date),
    pub title_game: Date,
    pub fcs_rounds: Vec<Date>,
    pub d2_rounds: Vec<Date>,
    pub juco_title: Date,
    pub fbs_regular_end: Date,
    pub fcs_regular_end: Date,
    pub d2_regular_end: Date,
    pub juco_regular_end: Date,

    /// The spring semester that opens in this football year's January (early enrollees arrive).
    pub spring_semester: Date,
    pub spring_practice: (Date, Date),
    pub spring_game: Date,
    pub spring_term_end: Date,
    pub summer_workouts: (Date, Date),
    pub money_year: Date,

    // --- Transfers, the draft choice and signing (winter of `season`) ---
    pub portal_open: Date,
    pub portal_close: Date,
    pub declare_soft: Date,
    pub declare_deadline: Date,
    pub declare_withdraw: Date,
    pub opt_out_deadline: Date,
    pub early_signing: Option<Date>,
    pub signing_day: Date,
    pub early_enrollment: Date,
    pub summer_enrollment: Date,

    // --- High school and recruiting ---
    pub hs_school_start: Date,
    pub junior_days: (Date, Date),
    pub all_american_bowl: Date,
    pub polynesian_bowl: Date,
    pub official_visits_from: Date,

    // --- Senior and all-star games (January of `season + 1`) ---
    pub shrine_bowl: Date,
    pub hbcu_bowl: Date,
    pub nflpa_bowl: Date,
    pub senior_bowl: Date,
}

/// Labor Day: the first Monday of September.
pub fn labor_day(y: i32) -> Date {
    Date::nth_weekday(y, 9, Weekday::Mon, 1)
}

/// Thanksgiving: the fourth Thursday of November.
pub fn thanksgiving(y: i32) -> Date {
    Date::nth_weekday(y, 11, Weekday::Thu, 4)
}

/// The NFL season opener: the Thursday after Labor Day.
pub fn nfl_kickoff(y: i32) -> Date {
    labor_day(y).add(3)
}

/// Regular-season weeks in the NFL by season: 16 games in 16 weeks to
/// 1989 (no byes), a bye week from 1990, 17 games from 2021.
pub fn nfl_regular_weeks(season: i32) -> usize {
    match season {
        ..=1989 => 16,
        1990..=1992 => 17,
        1993 => 18,
        1994..=1998 => 17,
        1999..=2001 => 17,
        2002..=2020 => 17,
        _ => 18,
    }
}

/// Real Super Bowl dates before 2004, when the season's start moved around
/// Labor Day weekend (seasons 1984-2003).
const EARLY_SUPER_BOWLS: [(i32, (i32, u32, u32)); 20] = [
    (1984, (1985, 1, 20)),
    (1985, (1986, 1, 26)),
    (1986, (1987, 1, 25)),
    (1987, (1988, 1, 31)),
    (1988, (1989, 1, 22)),
    (1989, (1990, 1, 28)),
    (1990, (1991, 1, 27)),
    (1991, (1992, 1, 26)),
    (1992, (1993, 1, 31)),
    (1993, (1994, 1, 30)),
    (1994, (1995, 1, 29)),
    (1995, (1996, 1, 28)),
    (1996, (1997, 1, 26)),
    (1997, (1998, 1, 25)),
    (1998, (1999, 1, 31)),
    (1999, (2000, 1, 30)),
    (2000, (2001, 1, 28)),
    (2001, (2002, 2, 3)),
    (2002, (2003, 1, 26)),
    (2003, (2004, 2, 1)),
];

/// Weeks between the last regular-season Sunday and the Super Bowl: five
/// (a bye week before the game), four in 1999 and 2001-2002.
fn sb_gap(season: i32) -> i32 {
    if matches!(season, 1999 | 2001 | 2002) { 28 } else { 35 }
}

/// The last regular-season Sunday of an NFL season.
fn nfl_last_sunday(season: i32, weeks: usize) -> Date {
    if let Some(&(_, (y, m, d))) = EARLY_SUPER_BOWLS.iter().find(|e| e.0 == season) {
        return Date::ymd(y, m, d).add(-sb_gap(season));
    }
    nfl_kickoff(season).on_or_after(Weekday::Sun).add(7 * (weeks as i32 - 1))
}

/// The Super Bowl of an NFL season.
pub fn super_bowl(season: i32) -> Date {
    nfl_last_sunday(season, nfl_regular_weeks(season)).add(sb_gap(season))
}

/// The week whose Tuesday holds the trade deadline: after week 6 to 2011,
/// week 8 from 2012, week 9 from 2024 (from memory, to verify).
fn trade_deadline_week(season: i32) -> usize {
    match season {
        ..=2011 => 6,
        2012..=2023 => 8,
        _ => 9,
    }
}

/// The negotiating window before free agency (2013 on, from memory): three
/// days to 2015, then two days from noon, the league year opening at 4pm ET
/// on the last day.
fn negotiating_window(season: i32, league_year_day: Date) -> Option<Moment> {
    match season {
        ..=2012 => None,
        2013..=2015 => Some(Moment::at(league_year_day.add(-3), 12, 0)),
        _ => Some(Moment::at(league_year_day.add(-2), 12, 0)),
    }
}

/// The Pro Bowl of a season, from its Super Bowl `sb` (from memory, to
/// verify): the Sunday after the Super Bowl to the 2008 season, the Sunday
/// before it from the 2009 season (Jan 31, 2010), none for the 2020 season
/// (cancelled), and the Tuesday of Super Bowl week from the 2025 season (Feb
/// 3, 2026; later years keep it, proposed, not confirmed).
fn pro_bowl(season: i32, sb: Date) -> Option<Date> {
    match season {
        ..=2008 => Some(sb.add(7)),
        2020 => None,
        2009..=2024 => Some(sb.add(-7)),
        _ => Some(sb.add(-5)),
    }
}

/// The East-West Shrine Bowl of a season, from its Super Bowl `sb` (from
/// memory, to verify): a Saturday a week before the Senior Bowl through the
/// January 2021 date, then a Thursday in Senior Bowl week from February 2022
/// (Las Vegas, then Frisco and Arlington).
fn shrine_bowl(season: i32, sb: Date) -> Date {
    if season <= 2020 { sb.add(-15) } else { sb.add(-10) }
}

/// The Senior Bowl of a season, from its Super Bowl `sb` (from memory, to
/// verify): the Saturday eight days before it, which falls in February in
/// the later Super Bowl years (Feb 5, 2022 to Feb 1, 2025; Jan 31, 2026).
fn senior_bowl(sb: Date) -> Date {
    sb.add(-8)
}

/// The first day of the two-day drafts before 1995 (from memory, to
/// verify): Tuesday and Wednesday to 1987, Sunday and Monday from 1988.
fn early_draft_day(year: i32) -> Option<Date> {
    let (m, d) = match year {
        1984 => (5, 1),
        1985 => (4, 30),
        1986 => (4, 29),
        1987 => (4, 28),
        1988 => (4, 24),
        1989 => (4, 23),
        1990 => (4, 22),
        1991 => (4, 21),
        1992 => (4, 26),
        1993 => (4, 25),
        1994 => (4, 24),
        _ => return None,
    };
    Some(Date::ymd(year, m, d))
}

/// The college postseason format for a season.
pub fn cfb_postseason(season: i32) -> CfbPostseason {
    match season {
        ..=1997 => CfbPostseason::PollsOnly,
        1998..=2013 => CfbPostseason::Bcs,
        2014..=2023 => CfbPostseason::FourTeam,
        _ => CfbPostseason::TwelveTeam,
    }
}

/// The first weekday on or after `d` that is not a weekend.
fn business_day(d: Date) -> Date {
    match d.weekday() {
        Weekday::Sat => d.add(2),
        Weekday::Sun => d.add(1),
        _ => d,
    }
}

impl YearDates {
    pub fn new(season: i32) -> YearDates {
        YearDates::with_weeks(season, nfl_regular_weeks(season), nfl_regular_weeks(season - 1))
    }

    /// A year whose NFL regular season runs `weeks` weeks, after one of
    /// `prev_weeks` (an 18-game season with two byes runs 20, T11 change 1):
    /// the Super Bowl moves with the last Sunday.
    pub fn with_weeks(season: i32, weeks: usize, prev_weeks: usize) -> YearDates {
        let s = season;
        let start = nfl_last_sunday(s - 1, prev_weeks).add(sb_gap(s - 1)).add(1);
        let sb = nfl_last_sunday(s, weeks).add(sb_gap(s));

        // --- NFL offseason: the new league year opens the second Wednesday of
        // March at 4pm ET (2026: March 11); the tag window opens 22 days
        // earlier and closes 8 days earlier (2026: Feb 17 to Mar 3); the
        // combine ends 9 days before the league year (2026: Feb 23 to Mar 2).
        let league_year_day = Date::nth_weekday(s, 3, Weekday::Wed, 2);
        let league_year = Moment::at(league_year_day, 16, 0);
        let combine_end = league_year_day.add(-9);
        let combine_start = combine_end.add(-7);
        // The draft: the fourth Thursday of April (2026: April 23-25, Pittsburgh).
        // Before 2010 it was a Saturday-Sunday weekend draft, and before 1995
        // a two-day draft on its own real days.
        let draft_thu = Date::nth_weekday(s, 4, Weekday::Thu, 4);
        let draft = if s >= 2010 {
            [Moment::at(draft_thu, 20, 0), Moment::at(draft_thu.add(1), 19, 0), Moment::at(draft_thu.add(2), 12, 0)]
        } else if let Some(first) = early_draft_day(s) {
            [Moment::at(first, 8, 0), Moment::at(first, 16, 0), Moment::at(first.add(1), 8, 0)]
        } else {
            let sat = draft_thu.add(2);
            [Moment::at(sat, 12, 0), Moment::at(sat, 18, 0), Moment::at(sat.add(1), 11, 0)]
        };
        let draft_end = draft[2].date;
        let last_sunday = nfl_last_sunday(s, weeks);
        let first_sunday = last_sunday.add(-7 * (weeks as i32 - 1));
        let kickoff = first_sunday.add(-3);
        let nfl_weeks: Vec<Date> = (0..weeks).map(|k| first_sunday.add(7 * k as i32)).collect();
        let trade_deadline = Moment::at(nfl_weeks[(trade_deadline_week(s) - 1).min(nfl_weeks.len() - 1)].add(2), 16, 0);
        let hof_game = kickoff.add(-35);

        // --- College: Week 1 is the Saturday before Labor Day, Week 0 a week
        // earlier (2026: Aug 29 and Sep 5); rivalry week is the Saturday after
        // Thanksgiving, and conference championships a week later.
        let ld = labor_day(s);
        let week1 = ld.add(-2);
        let week0 = week1.add(-7);
        let tg = thanksgiving(s);
        let rivalry = tg.add(2);
        let ccg = rivalry.add(7);
        let cfb_weeks: Vec<Date> = {
            let mut v = Vec::new();
            let mut d = week0;
            while d <= rivalry {
                v.push(d);
                d = d.add(7);
            }
            v
        };
        let selection = ccg.add(1);
        let army_navy = Date::nth_weekday(s, 12, Weekday::Sat, 2).max(ccg.add(7));
        let jan1 = Date::ymd(s + 1, 1, 1);
        let fmt = cfb_postseason(s);
        // The 12-team playoff (2026-27, source: CFP schedule of June 1, 2026):
        // first round Dec 18-19, quarterfinals Dec 30 and Jan 1, semifinals
        // Jan 14-15, the title game Monday Jan 25 in Las Vegas.
        let cfp_first = (selection.add(12), selection.add(13));
        let qf = (Date::ymd(s, 12, 30), jan1);
        let semis_thu = jan1.add(13).on_or_after(Weekday::Thu);
        let (semis, title) = match fmt {
            CfbPostseason::TwelveTeam if s == 2024 => {
                ((Date::ymd(2025, 1, 9), Date::ymd(2025, 1, 10)), Date::ymd(2025, 1, 20))
            }
            CfbPostseason::TwelveTeam if s == 2025 => {
                ((Date::ymd(2026, 1, 8), Date::ymd(2026, 1, 9)), Date::ymd(2026, 1, 19))
            }
            CfbPostseason::TwelveTeam => ((semis_thu, semis_thu.add(1)), semis_thu.add(11)),
            CfbPostseason::FourTeam => {
                let t = Date::ymd(s + 1, 1, 7).on_or_after(Weekday::Mon);
                ((Date::ymd(s, 12, 31), jan1), t)
            }
            CfbPostseason::Bcs => {
                let t =
                    if s >= 2006 { Date::ymd(s + 1, 1, 6).on_or_after(Weekday::Mon) } else { Date::ymd(s + 1, 1, 4) };
                ((jan1, Date::ymd(s + 1, 1, 2)), t)
            }
            CfbPostseason::PollsOnly => ((jan1, Date::ymd(s + 1, 1, 2)), Date::ymd(s + 1, 1, 3)),
        };
        // FCS: the first round the Saturday after the regular season (24
        // teams: Nov 28 in 2026), the title game in early January.
        let fcs_r1 = rivalry;
        let fcs_title = Date::ymd(s + 1, 1, 3).on_or_after(Weekday::Mon);
        let fcs_rounds = vec![fcs_r1, fcs_r1.add(7), fcs_r1.add(14), fcs_r1.add(21), fcs_title];
        // D2: 28 teams, from the Saturday before Thanksgiving to mid-December.
        let d2_r1 = tg.add(-5);
        let d2_rounds = vec![d2_r1, d2_r1.add(7), d2_r1.add(14), d2_r1.add(21), d2_r1.add(28)];
        // NJCAA: the national championship game, first Saturday of December.
        let juco_title = Date::nth_weekday(s, 12, Weekday::Sat, 1);

        // --- January: the portal (2026 on: one window, Jan 2-16) and the draft
        // deadlines (Jan 6 soft, Jan 15 hard, 72 hours to withdraw; a later
        // opt-out Feb 7 for players on title-game teams). Era windows differ;
        // `rules.rs` holds them.
        let portal_open = Date::ymd(s + 1, 1, 2);
        let portal_close = Date::ymd(s + 1, 1, 16);
        let declare_deadline = Date::ymd(s + 1, 1, 15);
        // Early signing, by the season it falls in: the third Wednesday of
        // December from 2017 to 2023 (Dec 20, 2017 to Dec 20, 2023), the first
        // Wednesday from 2024 (adopted March 2024; Dec 4, 2024 and Dec 2, 2026).
        // February signing is the first Wednesday of February.
        let early_signing = match s {
            ..=2016 => None,
            2017..=2023 => Some(Date::nth_weekday(s, 12, Weekday::Wed, 3)),
            _ => Some(Date::nth_weekday(s, 12, Weekday::Wed, 1)),
        };
        let signing_day = Date::nth_weekday(s + 1, 2, Weekday::Wed, 1);

        YearDates {
            season: s,
            start,
            super_bowl: sb,
            pro_bowl_prev: pro_bowl(s - 1, start.add(-1)).filter(|&d| d >= start),
            tag_window_open: league_year_day.add(-22),
            tag_deadline: Moment::at(league_year_day.add(-8), 16, 0),
            combine_start,
            combine_end,
            combine_invites: Date::nth_weekday(s, 2, Weekday::Fri, 1).max(start.add(1)),
            tampering_open: negotiating_window(s, league_year_day),
            league_year,
            roster_bonus_window: (league_year_day.add(2), league_year_day.add(5)),
            annual_meeting: (Date::last_weekday(s, 3, Weekday::Sun), Date::last_weekday(s, 3, Weekday::Sun).add(3)),
            comp_picks: Date::last_weekday(s, 3, Weekday::Sun).add(1),
            pro_days: (combine_end.add(2), draft_thu.add(-14)),
            offseason_new_coach: Date::nth_weekday(s, 4, Weekday::Mon, 1),
            offseason_returning: Date::nth_weekday(s, 4, Weekday::Mon, 3),
            top30_deadline: draft_thu.add(-8),
            rfa_deadline: draft_thu.add(-6),
            rfa_match_deadline: draft_thu.add(-1),
            draft,
            udfa_rush: Moment::at(draft_end, 18, 0),
            fifth_year_option: business_day(Date::ymd(s, 5, 1)),
            rookie_minicamp: draft_end.add(6),
            rookie_program: Date::nth_weekday(s, 5, Weekday::Mon, 2),
            rookie_premiere: (
                Date::nth_weekday(s, 5, Weekday::Wed, 2),
                Date::nth_weekday(s, 5, Weekday::Wed, 2).add(4),
            ),
            schedule_release: Date::nth_weekday(s, 5, Weekday::Wed, 2),
            spring_meeting: Date::nth_weekday(s, 5, Weekday::Tue, 3),
            otas: (Date::last_weekday(s, 5, Weekday::Mon).add(-7), Date::nth_weekday(s, 6, Weekday::Fri, 2)),
            june_1: Date::ymd(s, 6, 1),
            minicamp: (Date::nth_weekday(s, 6, Weekday::Tue, 2), Date::nth_weekday(s, 6, Weekday::Tue, 2).add(2)),
            ring_ceremony: Date::nth_weekday(s, 6, Weekday::Thu, 2),
            rookie_readiness: Date::last_weekday(s, 6, Weekday::Mon),
            summer_break: (Date::nth_weekday(s, 6, Weekday::Tue, 2).add(3), kickoff.add(-49).add(5)),
            tag_long_term_deadline: Moment::at(Date::ymd(s, 7, 15), 16, 0),
            rookies_report: kickoff.add(-56),
            veterans_report: kickoff.add(-49),
            hof_game,
            hof_enshrinement: hof_game.add(2),
            preseason: [kickoff.add(-28), kickoff.add(-21), kickoff.add(-14)],
            cutdown: Moment::at(kickoff.add(-9), 16, 0),
            waiver_claims: Moment::at(kickoff.add(-8), 12, 0),
            practice_squads: kickoff.add(-8),
            nfl_kickoff: kickoff,
            nfl_weeks,
            thanksgiving: tg,
            christmas: Date::ymd(s, 12, 25),
            trade_deadline,
            pro_bowl_voting: (
                Date::nth_weekday(s, 11, Weekday::Thu, 2),
                Date::ymd(s, 12, 15).on_or_after(Weekday::Mon),
            ),
            black_monday: last_sunday.add(1),
            wild_card: last_sunday.add(6),
            divisional: last_sunday.add(13),
            conference_title_games: last_sunday.add(21),
            nfl_honors: sb.add(-3),
            all_pro: last_sunday.add(5),
            hof_finalists: last_sunday.add(2),
            pro_bowl_games: pro_bowl(s, sb),
            labor_day: ld,
            coaches_preseason_poll: Date::nth_weekday(s, 8, Weekday::Mon, 1),
            ap_preseason_poll: Date::nth_weekday(s, 8, Weekday::Mon, 2),
            media_days: (Date::nth_weekday(s, 7, Weekday::Mon, 2), Date::last_weekday(s, 7, Weekday::Fri)),
            fall_camp: week0.add(-25),
            cfb_weeks,
            rivalry_week: rivalry,
            conference_championships: ccg,
            army_navy,
            selection_day: selection,
            first_cfp_ranking: Date::nth_weekday(s, 11, Weekday::Tue, 1),
            heisman_finalists: ccg.add(3),
            awards_show: ccg.add(6),
            heisman: Date::nth_weekday(s, 12, Weekday::Sat, 2).max(ccg.add(7)),
            all_america: ccg.add(5),
            finals_week: (Date::nth_weekday(s, 12, Weekday::Mon, 2), Date::nth_weekday(s, 12, Weekday::Mon, 2).add(4)),
            december_graduation: Date::nth_weekday(s, 12, Weekday::Sat, 2).add(7),
            bowl_season: (ccg.add(7).on_or_after(Weekday::Sat), Date::ymd(s + 1, 1, 3)),
            cfp_first_round: cfp_first,
            cfp_quarterfinals: qf,
            cfp_semifinals: semis,
            title_game: title,
            fcs_rounds,
            d2_rounds,
            juco_title,
            fbs_regular_end: rivalry,
            fcs_regular_end: rivalry.add(-7),
            d2_regular_end: tg.add(-12),
            juco_regular_end: Date::nth_weekday(s, 11, Weekday::Sat, 2),
            spring_semester: Date::ymd(s + 1, 1, 12),
            spring_practice: (Date::nth_weekday(s, 3, Weekday::Mon, 1), Date::nth_weekday(s, 4, Weekday::Sat, 3)),
            spring_game: Date::nth_weekday(s, 4, Weekday::Sat, 3),
            spring_term_end: Date::nth_weekday(s, 5, Weekday::Fri, 2),
            summer_workouts: (
                Date::nth_weekday(s, 6, Weekday::Mon, 1),
                Date::nth_weekday(s, 6, Weekday::Mon, 1).add(55),
            ),
            money_year: Date::ymd(s, 7, 1),
            portal_open,
            portal_close,
            declare_soft: Date::ymd(s + 1, 1, 6),
            declare_deadline,
            declare_withdraw: declare_deadline.add(3),
            opt_out_deadline: Date::ymd(s + 1, 2, 7),
            early_signing,
            signing_day,
            early_enrollment: Date::ymd(s + 1, 1, 10),
            summer_enrollment: Date::nth_weekday(s, 6, Weekday::Mon, 1),
            hs_school_start: Date::nth_weekday(s, 8, Weekday::Mon, 2),
            junior_days: (Date::ymd(s, 2, 1).max(start), Date::ymd(s, 3, 31)),
            all_american_bowl: Date::nth_weekday(s + 1, 1, Weekday::Sat, 2),
            polynesian_bowl: Date::nth_weekday(s + 1, 1, Weekday::Sat, 3),
            official_visits_from: Date::ymd(s, 4, 1),
            shrine_bowl: shrine_bowl(s, sb),
            hbcu_bowl: Date::last_weekday(s + 1, 1, Weekday::Sat).add(-7).max(title.add(5)),
            nflpa_bowl: Date::last_weekday(s + 1, 1, Weekday::Sat).add(-7).max(title.add(5)),
            senior_bowl: senior_bowl(sb),
        }
    }

    /// Whether `d` falls inside this football year.
    pub fn contains(&self, d: Date) -> bool {
        d >= self.start && d <= self.super_bowl
    }

    /// The college week number (0-based from Week 0) for a Saturday, if any.
    pub fn cfb_week(&self, d: Date) -> Option<usize> {
        self.cfb_weeks.iter().position(|&w| w == d)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_2026_anchors_match_the_real_calendar() {
        let y = YearDates::new(2026);
        assert_eq!(y.start, Date::ymd(2026, 2, 9), "day after Super Bowl LX (Feb 8, 2026)");
        assert_eq!(y.league_year.date, Date::ymd(2026, 3, 11));
        assert_eq!(y.tag_window_open, Date::ymd(2026, 2, 17));
        assert_eq!(y.tag_deadline.date, Date::ymd(2026, 3, 3));
        assert_eq!((y.combine_start, y.combine_end), (Date::ymd(2026, 2, 23), Date::ymd(2026, 3, 2)));
        assert_eq!(y.tampering_open.map(|m| m.date), Some(Date::ymd(2026, 3, 9)));
        assert_eq!(y.annual_meeting.0, Date::ymd(2026, 3, 29));
        assert_eq!(y.offseason_new_coach, Date::ymd(2026, 4, 6));
        assert_eq!(y.offseason_returning, Date::ymd(2026, 4, 20));
        assert_eq!(y.top30_deadline, Date::ymd(2026, 4, 15));
        assert_eq!(y.rfa_deadline, Date::ymd(2026, 4, 17));
        assert_eq!(y.draft[0].date, Date::ymd(2026, 4, 23));
        assert_eq!(y.fifth_year_option, Date::ymd(2026, 5, 1));
        assert_eq!(y.rookie_program, Date::ymd(2026, 5, 11));
        assert_eq!(y.nfl_kickoff, Date::ymd(2026, 9, 10));
        assert_eq!(y.cfb_weeks[0], Date::ymd(2026, 8, 29), "Week 0");
        assert_eq!(y.rivalry_week, Date::ymd(2026, 11, 28));
        assert_eq!(y.conference_championships, Date::ymd(2026, 12, 5));
        assert_eq!(y.selection_day, Date::ymd(2026, 12, 6));
        assert_eq!(y.army_navy, Date::ymd(2026, 12, 12));
        assert_eq!(y.bowl_season.0, Date::ymd(2026, 12, 12), "the Saturday after the title games");
        assert_eq!(y.cfp_first_round, (Date::ymd(2026, 12, 18), Date::ymd(2026, 12, 19)));
        assert_eq!(y.cfp_semifinals, (Date::ymd(2027, 1, 14), Date::ymd(2027, 1, 15)));
        assert_eq!(y.title_game, Date::ymd(2027, 1, 25));
        assert_eq!(y.early_signing, Some(Date::ymd(2026, 12, 2)));
        assert_eq!(y.signing_day, Date::ymd(2027, 2, 3));
        assert_eq!(y.super_bowl, Date::ymd(2027, 2, 14), "Super Bowl LXI");
        assert_eq!(y.nfl_weeks.len(), 18);
        assert_eq!(y.trade_deadline.date, Date::ymd(2026, 11, 10));
    }

    /// The early signing period by season: none before 2017, the third
    /// Wednesday of December through 2023, the first Wednesday from 2024.
    #[test]
    fn early_signing_follows_the_rule_of_its_season() {
        assert_eq!(YearDates::new(2016).early_signing, None);
        assert_eq!(YearDates::new(2017).early_signing, Some(Date::ymd(2017, 12, 20)));
        assert_eq!(YearDates::new(2022).early_signing, Some(Date::ymd(2022, 12, 21)));
        assert_eq!(YearDates::new(2023).early_signing, Some(Date::ymd(2023, 12, 20)));
        assert_eq!(YearDates::new(2024).early_signing, Some(Date::ymd(2024, 12, 4)));
        assert_eq!(YearDates::new(2025).early_signing, Some(Date::ymd(2025, 12, 3)));
    }

    #[test]
    fn past_super_bowls_land_on_their_real_sundays() {
        for (season, (y, m, d)) in [
            (1996, (1997, 1, 26)),
            (2002, (2003, 1, 26)),
            (2007, (2008, 2, 3)),
            (2019, (2020, 2, 2)),
            (2021, (2022, 2, 13)),
            (2024, (2025, 2, 9)),
            (2025, (2026, 2, 8)),
        ] {
            assert_eq!(super_bowl(season), Date::ymd(y, m, d), "season {season}");
        }
    }

    /// The NFL's dated steps by era (from memory, to verify): the trade
    /// deadline after week 8 (2012-2023) and week 9 (2024 on), the
    /// negotiating window from 2013, the Pro Bowl after the Super Bowl to the
    /// 2008 season and before it from 2009, the Shrine Bowl on a Saturday,
    /// then a Thursday from 2022, the Senior Bowl eight days before the Super
    /// Bowl.
    #[test]
    fn nfl_steps_fall_on_their_real_days_by_era() {
        let deadline = |s: i32| YearDates::new(s).trade_deadline.date;
        assert_eq!(deadline(2013), Date::ymd(2013, 10, 29));
        assert_eq!(deadline(2023), Date::ymd(2023, 10, 31));
        assert_eq!(deadline(2024), Date::ymd(2024, 11, 5));
        assert!(YearDates::new(2012).tampering_open.is_none());
        let y2014 = YearDates::new(2014);
        assert_eq!(y2014.tampering_open.map(|m| m.date), Some(y2014.league_year.date.add(-3)), "three days to 2015");
        assert_eq!(YearDates::new(2008).pro_bowl_games, Some(Date::ymd(2009, 2, 8)));
        assert_eq!(YearDates::new(2009).pro_bowl_prev, Some(Date::ymd(2009, 2, 8)), "inside the next year");
        assert_eq!(YearDates::new(2009).pro_bowl_games, Some(Date::ymd(2010, 1, 31)));
        assert_eq!(YearDates::new(2010).pro_bowl_prev, None);
        assert_eq!(YearDates::new(2020).pro_bowl_games, None);
        assert_eq!(YearDates::new(2025).pro_bowl_games, Some(Date::ymd(2026, 2, 3)));
        assert_eq!(YearDates::new(2019).shrine_bowl, Date::ymd(2020, 1, 18));
        assert_eq!(YearDates::new(2021).shrine_bowl, Date::ymd(2022, 2, 3));
        assert_eq!(YearDates::new(2024).shrine_bowl, Date::ymd(2025, 1, 30));
        assert_eq!(YearDates::new(2014).senior_bowl, Date::ymd(2015, 1, 24));
        assert_eq!(YearDates::new(2021).senior_bowl, Date::ymd(2022, 2, 5));
        assert_eq!(YearDates::new(2024).senior_bowl, Date::ymd(2025, 2, 1));
        assert_eq!(YearDates::new(2025).senior_bowl, Date::ymd(2026, 1, 31));
        // Every college all-star game comes after the title game, so the
        // finalists' seniors can play in them.
        for s in 1996..=2060 {
            let y = YearDates::new(s);
            for g in [y.shrine_bowl, y.hbcu_bowl, y.nflpa_bowl, y.senior_bowl] {
                assert!(g > y.title_game.add(1), "season {s}");
            }
        }
    }

    /// An 18-game season (T11 change 1) runs 19 weeks and moves the Super
    /// Bowl a week later; the next year starts the day after it.
    #[test]
    fn a_longer_nfl_season_moves_the_super_bowl() {
        let base = YearDates::new(2027);
        let long = YearDates::with_weeks(2027, 19, 18);
        assert_eq!(long.nfl_weeks.len(), 19);
        assert_eq!(long.super_bowl, base.super_bowl.add(7));
        let next = YearDates::with_weeks(2028, 19, 19);
        assert_eq!(next.start, long.super_bowl.add(1));
    }

    #[test]
    fn dates_round_trip() {
        for days in -20_000..40_000 {
            let d = Date(days);
            let (y, m, dd) = d.parts();
            assert_eq!(Date::ymd(y, m, dd), d);
        }
        assert_eq!(Date::ymd(2026, 10, 15).weekday(), Weekday::Thu);
    }
}
