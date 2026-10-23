//! The rule book for every level and every year (topic 11).
//!
//! The 30 hidden years before 2026 use each era's real off-field rules. From
//! 2026 on the on-field rules never change (Raichel: "no because it may cause
//! engine breaking aspects that i can't fix in game"), and off-field rules
//! change only through the fixed list of 12 agreed possibilities, each by its
//! real process. Their chances are Claude's proposal: proposed, not confirmed.
//!
//! Sources for the era rules (checked when building, 2026-10-15): the NFL's
//! 17-game season (2021) and 14-team playoff (2020); the BCS (1998-2013), the
//! 4-team CFP (2014-2023) and the 12-team CFP (2024); the FCS playoff field
//! (16 to 2009, 20 to 2012, 24 since 2013); D2 (24 to 2015, 28 since 2016);
//! the transfer portal (October 2018), the one-time transfer exception (2021),
//! portal windows (2021-22 to 2025-26), one window Jan 2-16 (2026); NIL (July
//! 1, 2021); revenue sharing (July 1, 2025, about $20.5M); the early signing
//! period (December 2017); official visits from April 1 of the junior year
//! (2018); the 4-game redshirt rule (2018); test scores dropped from NCAA
//! initial eligibility (waived for the 2021-2022 classes, removed 2023).

use super::anchors::{CfbPostseason, cfb_postseason, nfl_regular_weeks};
use crate::rng::Rng;

/// How players change schools in an era.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TransferRule {
    /// Before the portal: a transfer needs his school's release and sits a year.
    Permission,
    /// The portal (opened October 2018, seasons 2018-2021): enter any time;
    /// a transfer sat a year until the one-time rule of 2021.
    OpenPortal,
    /// Windows (seasons 2022-2024): a winter window after the season and a
    /// spring window, one free transfer.
    Windows,
    /// One January window, Jan 2-16 (from the 2025 season's offseason).
    January,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RuleBook {
    pub season: i32,
    // --- NFL ---
    pub nfl_games: u32,
    pub nfl_weeks: u32,
    pub nfl_playoff_teams: u32,
    /// The regular-season roster limit: 49 in 1984, 45 in 1985-88, 47 in
    /// 1989-92, 53 from 1993 (real).
    pub nfl_roster: u32,
    /// Players dressed on game day: everyone through 1988, 45 to 2010 (a
    /// third quarterback could dress as an emergency), 46 from 2011, 48
    /// from 2020 (with eight offensive linemen).
    pub nfl_actives: u32,
    /// The offseason roster limit: 80 to 2010, 90 from 2011.
    pub nfl_offseason: u32,
    pub nfl_practice_squad: u32,
    /// CBA: padded practices allowed in the regular season (14 since 2011).
    pub nfl_padded_practices: u32,
    /// Salary cap in dollars (None: no cap before 1994, and the uncapped 2010 year).
    pub salary_cap: Option<f64>,
    /// A lockout froze the offseason this year (until this many days after the new league year).
    pub lockout_days: Option<u32>,
    // --- College ---
    pub cfb_postseason: CfbPostseason,
    pub cfp_teams: u32,
    pub fbs_games: u32,
    pub fcs_playoff: u32,
    pub d2_playoff: u32,
    pub transfer: TransferRule,
    pub one_time_transfer: bool,
    pub nil: bool,
    pub revenue_sharing: Option<f64>,
    pub early_signing: bool,
    pub junior_officials: bool,
    pub four_game_redshirt: bool,
    /// 5 years to play 5 (agreed change 9).
    pub five_in_five: bool,
    /// Test scores count toward NCAA initial eligibility for this signing class.
    pub test_scores: bool,
    /// The SEC and Big Ten play their own postseason (agreed change 8).
    pub breakaway: bool,
}

/// Real NFL salary caps, 1994-2026, in millions (2010 was uncapped). 2026:
/// $301.2M as reported in February 2026 (to check against the final figure).
const SALARY_CAP: [(i32, f64); 33] = [
    (1994, 34.608),
    (1995, 37.1),
    (1996, 40.753),
    (1997, 41.454),
    (1998, 52.388),
    (1999, 57.288),
    (2000, 62.172),
    (2001, 67.405),
    (2002, 71.101),
    (2003, 75.007),
    (2004, 80.582),
    (2005, 85.5),
    (2006, 102.0),
    (2007, 109.0),
    (2008, 116.0),
    (2009, 123.0),
    (2010, 0.0),
    (2011, 120.375),
    (2012, 120.6),
    (2013, 123.0),
    (2014, 133.0),
    (2015, 143.28),
    (2016, 155.27),
    (2017, 167.0),
    (2018, 177.2),
    (2019, 188.2),
    (2020, 198.2),
    (2021, 182.5),
    (2022, 208.2),
    (2023, 224.8),
    (2024, 255.4),
    (2025, 279.2),
    (2026, 301.2),
];

/// The cap after 2026 (section 5 topic 2, Cain's rule): 8% a year, and in
/// a year a new media deal starts, the average of the last three media-deal
/// jumps (real: 2006 +19.3%, 2014 +8.1%, 2022 +14.1%). The deals run eight
/// years like the real ones, so the next starts in 2030 (proposed, not
/// confirmed); the average rolls forward with the world's own deals.
pub fn future_cap(season: i32) -> f64 {
    let mut cap = 301.2e6;
    let mut jumps = [0.193, 0.081, 0.141];
    for y in 2027..=season {
        if (y - 2022) % 8 == 0 {
            let jump = jumps.iter().sum::<f64>() / 3.0;
            jumps = [jumps[1], jumps[2], jump];
            cap *= 1.0 + jump;
        } else {
            cap *= 1.08;
        }
    }
    cap
}

/// Whether a new media deal starts this season (real 1998, 2006, 2014, 2022; then every eight years).
pub fn media_deal_year(season: i32) -> bool {
    matches!(season, 1998 | 2006 | 2014) || (season >= 2022 && (season - 2022) % 8 == 0)
}

/// The real cap for a year, if known.
pub fn real_salary_cap(year: i32) -> Option<Option<f64>> {
    SALARY_CAP.iter().find(|c| c.0 == year).map(|c| if c.1 == 0.0 { None } else { Some(c.1 * 1.0e6) })
}

/// Conference games and whether a conference holds a title game, by season.
/// The world uses today's conference line-ups for every year; the counts and
/// title-game years follow each conference's real history.
pub fn conference_rules(conf: &str, season: i32) -> (u32, bool) {
    match conf {
        "SEC" => (if season >= 2026 { 9 } else { 8 }, season >= 1992),
        "Big Ten" => (if season >= 2016 { 9 } else { 8 }, season >= 2011),
        "Big 12" => (if season >= 2011 { 9 } else { 8 }, (1996..=2010).contains(&season) || season >= 2017),
        "ACC" => (8, season >= 2005),
        "Pac-12" => (7, season >= 2011),
        "American" => (8, season >= 2015),
        "CUSA" => (8, season >= 2005),
        "MAC" => (8, season >= 1997),
        "Mountain West" => (8, season >= 2013),
        "Sun Belt" => (8, season >= 2018),
        "SWAC" => (8, season >= 1999),
        _ => (8, false),
    }
}

impl RuleBook {
    /// The real rule book of a season before any agreed change.
    pub fn era(season: i32) -> RuleBook {
        let weeks = nfl_regular_weeks(season) as u32;
        // The cap began in 1994; later years grow from 2026's.
        let cap = match real_salary_cap(season) {
            Some(c) => c,
            None if season < 1994 => None,
            None => Some(future_cap(season)),
        };
        let fmt = cfb_postseason(season);
        RuleBook {
            season,
            // 1987: the strike cancelled week 3, so 15 games.
            nfl_games: match season {
                1987 => 15,
                ..=2020 => 16,
                _ => 17,
            },
            nfl_weeks: weeks,
            nfl_playoff_teams: match season {
                ..=1989 => 10,
                1990..=2019 => 12,
                _ => 14,
            },
            nfl_roster: match season {
                ..=1984 => 49,
                1985..=1988 => 45,
                1989..=1992 => 47,
                _ => 53,
            },
            nfl_actives: match season {
                ..=1984 => 49,
                1985..=1988 => 45,
                1989..=2010 => 45,
                2011..=2019 => 46,
                _ => 48,
            },
            nfl_offseason: if season >= 2011 { 90 } else { 80 },
            // Practice squad spots: none before the 1989 developmental
            // squad, 5 to 2003, 8 from 2004, 10 from 2014, 16
            // in the COVID years 2020-2021 (the new CBA's 12 was raised) and
            // kept from 2022 (from memory, to verify); 17 from 2025 as first
            // built.
            nfl_practice_squad: match season {
                ..=1988 => 0,
                1989..=2003 => 5,
                2004..=2013 => 8,
                2014..=2019 => 10,
                2020..=2021 => 16,
                2022..=2024 => 16,
                _ => 17,
            },
            nfl_padded_practices: if season >= 2011 { 14 } else { 99 },
            salary_cap: cap,
            lockout_days: if season == 2011 { Some(136) } else { None },
            cfb_postseason: fmt,
            cfp_teams: match fmt {
                CfbPostseason::PollsOnly => 0,
                CfbPostseason::Bcs => 2,
                CfbPostseason::FourTeam => 4,
                CfbPostseason::TwelveTeam => 12,
            },
            fbs_games: if season >= 2006 { 12 } else { 11 },
            fcs_playoff: match season {
                ..=2009 => 16,
                2010..=2012 => 20,
                _ => 24,
            },
            d2_playoff: if season >= 2016 { 28 } else { 24 },
            transfer: match season {
                ..=2017 => TransferRule::Permission,
                2018..=2021 => TransferRule::OpenPortal,
                2022..=2024 => TransferRule::Windows,
                _ => TransferRule::January,
            },
            one_time_transfer: season >= 2021,
            nil: season >= 2021,
            revenue_sharing: if season >= 2025 { Some(20.5e6 * 1.04f64.powi(season - 2025)) } else { None },
            early_signing: season + 1 >= 2018,
            junior_officials: season >= 2018,
            // The four-game redshirt (2018) ends with redshirts themselves
            // when Division I moves to five years to play five (S11: adopted
            // by the Division I Cabinet on June 23-24, 2026, for 2026-27).
            four_game_redshirt: (2018..crate::world::eligibility::FIVE_IN_FIVE_FROM).contains(&season),
            five_in_five: season >= crate::world::eligibility::FIVE_IN_FIVE_FROM,
            test_scores: season < 2020,
            breakaway: false,
        }
    }
}

/// The 12 agreed off-field changes (topic 11, confirmed 2026-10-14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Change {
    EighteenGames,
    NewCba,
    SixteenTeamPlayoff,
    RosterSize,
    Relocation,
    BiggerCfp,
    ConferenceSwitch,
    Breakaway,
    FiveInFive,
    RevenueCap,
    HsNil,
    HsTransferRules,
}

impl Change {
    pub const ALL: [Change; 12] = [
        Change::EighteenGames,
        Change::NewCba,
        Change::SixteenTeamPlayoff,
        Change::RosterSize,
        Change::Relocation,
        Change::BiggerCfp,
        Change::ConferenceSwitch,
        Change::Breakaway,
        Change::FiveInFive,
        Change::RevenueCap,
        Change::HsNil,
        Change::HsTransferRules,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Change::EighteenGames => "18-game NFL season",
            Change::NewCba => "new CBA or lockout",
            Change::SixteenTeamPlayoff => "16-team NFL playoff",
            Change::RosterSize => "roster or practice squad size",
            Change::Relocation => "team relocation",
            Change::BiggerCfp => "bigger college playoff",
            Change::ConferenceSwitch => "conference switch",
            Change::Breakaway => "SEC and Big Ten breakaway",
            Change::FiveInFive => "5 years to play 5",
            Change::RevenueCap => "revenue-sharing cap change",
            Change::HsNil => "high school NIL",
            Change::HsTransferRules => "high school transfer rules",
        }
    }
    /// The real process the change goes through, for the news and trials.
    pub fn process(self) -> &'static str {
        match self {
            Change::EighteenGames => "agreed in CBA talks, then an owners' vote (24 of 32)",
            Change::NewCba => "the CBA ends after the 2030 season: a deal or a lockout",
            Change::SixteenTeamPlayoff => "owners' vote at the March annual meeting (24 of 32)",
            Change::RosterSize => "owners' vote at the March annual meeting (24 of 32), with the union",
            Change::Relocation => "owners' vote (24 of 32) on the team's request",
            Change::BiggerCfp => "the CFP management committee (conference commissioners)",
            Change::ConferenceSwitch => "a school accepts a conference's invitation",
            Change::Breakaway => "the SEC and Big Ten leave the CFP for their own postseason",
            Change::FiveInFive => {
                "the NCAA Division I Cabinet's vote of June 23-24, 2026 (real; in force from 2026-27)"
            }
            Change::RevenueCap => "the House settlement's yearly review or a court ruling",
            Change::HsNil => "a state association or legislature vote",
            Change::HsTransferRules => "a state association vote",
        }
    }
}

/// Yearly chances for the 12 changes from 2027 on (proposed, not confirmed;
/// Raichel was asked for them on 2026-10-14 and has not answered). Each is
/// the chance the change is put forward in a year; passing it is a separate
/// real vote where one exists.
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct ChangeOdds {
    /// 18 games, only inside a new CBA (from the 2031 league year).
    pub eighteen_games_in_cba: f64,
    /// A lockout when the CBA ends after 2030 (2011 was the last).
    pub lockout: f64,
    pub sixteen_team_proposal: f64,
    pub roster_size_proposal: f64,
    pub relocation_request: f64,
    pub bigger_cfp: f64,
    pub conference_switch: f64,
    pub breakaway: f64,
    pub five_in_five: f64,
    pub revenue_cap_ruling: f64,
    /// Per state without high school NIL.
    pub hs_nil_per_state: f64,
    /// Per state.
    pub hs_transfer_per_state: f64,
    /// Chance an owner votes yes on a league proposal.
    pub owner_yes: f64,
}

pub const CHANGE_ODDS: ChangeOdds = ChangeOdds {
    eighteen_games_in_cba: 0.55,
    lockout: 0.25,
    sixteen_team_proposal: 0.10,
    roster_size_proposal: 0.15,
    relocation_request: 0.015,
    bigger_cfp: 0.25,
    conference_switch: 0.10,
    breakaway: 0.02,
    // Real since June 2026 (S11): in the rule book, never rolled.
    five_in_five: 0.0,
    revenue_cap_ruling: 0.10,
    hs_nil_per_state: 0.06,
    hs_transfer_per_state: 0.02,
    owner_yes: 0.72,
};

/// One change that happened, for the news and the trial.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct AppliedChange {
    pub change: Change,
    pub season: i32,
    pub detail: String,
}

/// The state of the 12 changes in a world.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChangeState {
    pub applied: Vec<AppliedChange>,
    pub eighteen: Option<i32>,
    pub sixteen: Option<i32>,
    pub roster_delta: i32,
    pub squad_delta: i32,
    pub cfp_size: Option<(i32, u32)>,
    pub breakaway: Option<i32>,
    pub five_in_five: Option<i32>,
    pub revenue_factor: f64,
    pub cba_resolved: bool,
    pub lockout: Option<(i32, u32)>,
    /// A failed owners' vote, for the news: (season, change, yes votes).
    pub failed_votes: Vec<(i32, Change, u32)>,
}

impl ChangeState {
    /// The rule book of a season with every change that has happened.
    pub fn rules(&self, season: i32) -> RuleBook {
        let mut r = RuleBook::era(season);
        if self.eighteen.is_some_and(|y| season >= y) {
            r.nfl_games = 18;
            r.nfl_weeks = 20;
        }
        if self.sixteen.is_some_and(|y| season >= y) {
            r.nfl_playoff_teams = 16;
        }
        // The floors bound a voted change; they never move an era's real size
        // (five practice squad spots to 2003).
        if self.roster_delta != 0 {
            r.nfl_roster = (r.nfl_roster as i32 + self.roster_delta).max(45) as u32;
        }
        if self.squad_delta != 0 {
            r.nfl_practice_squad = (r.nfl_practice_squad as i32 + self.squad_delta).max(8) as u32;
        }
        if let Some((y, n)) = self.cfp_size
            && season >= y
        {
            r.cfp_teams = n;
        }
        if self.breakaway.is_some_and(|y| season >= y) {
            r.breakaway = true;
        }
        if self.five_in_five.is_some_and(|y| season >= y) {
            r.five_in_five = true;
        }
        if let Some(cap) = r.revenue_sharing.as_mut()
            && self.revenue_factor > 0.0
        {
            *cap *= self.revenue_factor;
        }
        if let Some((y, days)) = self.lockout
            && y == season
        {
            r.lockout_days = Some(days);
        }
        r
    }

    /// An owners' vote: 32 owners, 24 needed.
    fn owners_vote(rng: &mut Rng, p: f64) -> u32 {
        (0..32).filter(|_| rng.chance(p)).count() as u32
    }

    /// Rolls the NFL changes at the annual league meeting of `season` (late
    /// March). Returns the news lines.
    pub fn league_meeting(&mut self, season: i32, rng: &mut Rng) -> Vec<String> {
        let mut news = Vec::new();
        if season < 2027 {
            return news;
        }
        let odds = CHANGE_ODDS;
        // The CBA runs through the 2030 season: the new deal lands in March 2031.
        if season >= 2031 && !self.cba_resolved {
            self.cba_resolved = true;
            if rng.chance(odds.lockout) {
                let days = rng.int(40, 160) as u32;
                self.lockout = Some((season, days));
                news.push(format!(
                    "The CBA has expired and the owners lock the players out: no free agency, trades or offseason work for {days} days."
                ));
                self.applied.push(AppliedChange {
                    change: Change::NewCba,
                    season,
                    detail: format!("lockout of {days} days, then a new CBA"),
                });
            } else {
                news.push("The league and the union agree on a new CBA before the old one runs out.".into());
                self.applied.push(AppliedChange {
                    change: Change::NewCba,
                    season,
                    detail: "new CBA without a lockout".into(),
                });
            }
            if rng.chance(odds.eighteen_games_in_cba) {
                let yes = Self::owners_vote(rng, 0.85);
                if yes >= 24 {
                    // The schedule for the coming season is already set: the
                    // longer season starts the year after.
                    self.eighteen = Some(season + 1);
                    news.push(format!(
                        "The new CBA brings an 18-game season with two byes from {}, approved by the owners {yes}-{}.",
                        season + 1,
                        32 - yes
                    ));
                    self.applied.push(AppliedChange {
                        change: Change::EighteenGames,
                        season,
                        detail: format!("owners voted {yes}-{}", 32 - yes),
                    });
                } else {
                    self.failed_votes.push((season, Change::EighteenGames, yes));
                }
            }
        }
        if self.sixteen.is_none() && rng.chance(odds.sixteen_team_proposal) {
            let yes = Self::owners_vote(rng, odds.owner_yes);
            if yes >= 24 {
                self.sixteen = Some(season);
                news.push(format!("Owners vote {yes}-{} to grow the playoffs to 16 teams.", 32 - yes));
                self.applied.push(AppliedChange {
                    change: Change::SixteenTeamPlayoff,
                    season,
                    detail: format!("owners voted {yes}-{}", 32 - yes),
                });
            } else {
                self.failed_votes.push((season, Change::SixteenTeamPlayoff, yes));
                news.push(format!("A proposal for a 16-team playoff fails, {yes} votes of the 24 needed."));
            }
        }
        if rng.chance(odds.roster_size_proposal) {
            let yes = Self::owners_vote(rng, 0.8);
            if yes >= 24 {
                let squad = rng.chance(0.6);
                let delta = if rng.chance(0.75) { 1 } else { -1 };
                let what = if squad {
                    self.squad_delta += delta;
                    "practice squad"
                } else {
                    self.roster_delta += delta;
                    "active roster"
                };
                let dir = if delta > 0 { "grows" } else { "shrinks" };
                news.push(format!("The {what} {dir} by one spot after an owners' vote ({yes}-{}).", 32 - yes));
                self.applied.push(AppliedChange {
                    change: Change::RosterSize,
                    season,
                    detail: format!("{what} {dir} by one"),
                });
            } else {
                self.failed_votes.push((season, Change::RosterSize, yes));
            }
        }
        news
    }

    /// Rolls the college changes once a year (summer, before the season).
    pub fn college_year(&mut self, season: i32, rng: &mut Rng) -> Vec<(Change, String)> {
        let mut out = Vec::new();
        if season < 2027 {
            return out;
        }
        let odds = CHANGE_ODDS;
        if self.cfp_size.is_none() && rng.chance(odds.bigger_cfp) {
            let n = if rng.chance(0.6) { 16 } else { 14 };
            self.cfp_size = Some((season, n));
            out.push((Change::BiggerCfp, format!("The College Football Playoff grows to {n} teams from this season.")));
            self.applied.push(AppliedChange { change: Change::BiggerCfp, season, detail: format!("{n} teams") });
        }
        if self.breakaway.is_none() && season >= 2028 && rng.chance(odds.breakaway) {
            self.breakaway = Some(season);
            out.push((Change::Breakaway, "The SEC and Big Ten break away and crown their own champion.".into()));
            self.applied.push(AppliedChange { change: Change::Breakaway, season, detail: "own postseason".into() });
        }
        // Five years to play five is no longer a possible change: it became
        // real in June 2026 (S11) and is in every rule book from 2026-27.
        if rng.chance(odds.revenue_cap_ruling) {
            let up = rng.chance(0.65);
            let f = if up { rng.range(1.05, 1.25) } else { rng.range(0.8, 0.95) };
            self.revenue_factor = if self.revenue_factor > 0.0 { self.revenue_factor * f } else { f };
            let dir = if up { "raises" } else { "lowers" };
            out.push((
                Change::RevenueCap,
                format!("A ruling {dir} the revenue-sharing cap by {:.0}%.", (f - 1.0).abs() * 100.0),
            ));
            self.applied.push(AppliedChange { change: Change::RevenueCap, season, detail: format!("x{f:.2}") });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn era_rules_change_in_the_right_years() {
        assert_eq!(RuleBook::era(2020).nfl_games, 16);
        assert_eq!(RuleBook::era(2021).nfl_games, 17);
        assert_eq!(RuleBook::era(2019).nfl_playoff_teams, 12);
        assert_eq!(RuleBook::era(2020).nfl_playoff_teams, 14);
        assert_eq!(RuleBook::era(1997).cfp_teams, 0);
        assert_eq!(RuleBook::era(2005).cfp_teams, 2);
        assert_eq!(RuleBook::era(2014).cfp_teams, 4);
        assert_eq!(RuleBook::era(2024).cfp_teams, 12);
        assert_eq!(RuleBook::era(2017).transfer, TransferRule::Permission);
        assert_eq!(RuleBook::era(2018).transfer, TransferRule::OpenPortal);
        assert_eq!(RuleBook::era(2026).transfer, TransferRule::January);
        assert!(!RuleBook::era(2020).nil && RuleBook::era(2021).nil);
        assert!(RuleBook::era(2025).revenue_sharing.is_some() && RuleBook::era(2024).revenue_sharing.is_none());
        assert_eq!(RuleBook::era(2010).salary_cap, None);
        assert_eq!(RuleBook::era(1993).salary_cap, None, "no cap before 1994");
        assert!(RuleBook::era(1994).salary_cap.is_some() && RuleBook::era(2030).salary_cap.is_some());
        assert_eq!(RuleBook::era(2012).fcs_playoff, 20);
        assert_eq!(RuleBook::era(2013).fcs_playoff, 24);
        assert!(!RuleBook::era(2016).early_signing && RuleBook::era(2017).early_signing);
    }

    /// Practice squads keep each era's real size until a vote changes it.
    #[test]
    fn practice_squads_keep_their_real_size_until_a_vote() {
        let none = ChangeState::default();
        assert_eq!(none.rules(1995).nfl_practice_squad, 5);
        assert_eq!(none.rules(2004).nfl_practice_squad, 8);
        assert_eq!(none.rules(2014).nfl_practice_squad, 10);
        assert_eq!(none.rules(2022).nfl_practice_squad, 16);
        let voted = ChangeState { squad_delta: 1, ..ChangeState::default() };
        assert_eq!(voted.rules(2030).nfl_practice_squad, none.rules(2030).nfl_practice_squad + 1);
    }
}
