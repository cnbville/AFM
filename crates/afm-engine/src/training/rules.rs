//! The year and its rules (topic 3): each level's blocks on its real
//! calendar, the share of a year's growth each block holds, and the real
//! rule books by level, block and era, with the checks a practice plan
//! must pass before it is saved.

use super::FAMILIES;
use super::plan::{Contact, Day, Gear, Kind, Week};
use crate::calendar::{Date, Weekday, YearDates};

/// The level a team trains under.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Tier {
    High,
    College,
    Pro,
}

/// A part of the training year (calendar topic 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[repr(u8)]
pub enum Block {
    /// College winter conditioning, the NFL's weeks before the offseason
    /// program, high school's offseason lifting.
    Winter,
    /// College spring practice, high school spring ball.
    Spring,
    /// College weeks after spring with nothing required.
    Quiet,
    /// College's summer hours, high school's summer program.
    Summer,
    Camp,
    Season,
    /// College bowl practice and the weeks to January.
    Postseason,
    /// NFL offseason program: phase 1 (strength only), phase 2 (no offense
    /// against defense), OTAs and minicamp (phase 3), and the summer break.
    Phase1,
    Phase2,
    Otas,
    Break,
}

pub const BLOCKS: usize = 11;

impl Block {
    pub const ALL: [Block; BLOCKS] = [
        Block::Winter,
        Block::Spring,
        Block::Quiet,
        Block::Summer,
        Block::Camp,
        Block::Season,
        Block::Postseason,
        Block::Phase1,
        Block::Phase2,
        Block::Otas,
        Block::Break,
    ];

    pub fn name(self) -> &'static str {
        ["winter", "spring", "quiet", "summer", "camp", "season", "postseason", "phase1", "phase2", "otas", "break"]
            [self as usize]
    }

    pub fn label(self, tier: Tier) -> &'static str {
        match (self, tier) {
            (Block::Winter, Tier::Pro) => "Offseason (on their own)",
            (Block::Winter, Tier::College) => "Winter conditioning",
            (Block::Winter, Tier::High) => "Offseason program",
            (Block::Spring, Tier::High) => "Spring ball",
            (Block::Spring, _) => "Spring practice",
            (Block::Quiet, _) => "Quiet weeks (on their own)",
            (Block::Summer, Tier::College) => "Summer program",
            (Block::Summer, _) => "Summer workouts",
            (Block::Camp, Tier::Pro) => "Training camp",
            (Block::Camp, Tier::College) => "Fall camp",
            (Block::Camp, Tier::High) => "Preseason practice",
            (Block::Season, _) => "Season",
            (Block::Postseason, _) => "Bowl practice and December",
            (Block::Phase1, _) => "Offseason program, phase 1",
            (Block::Phase2, _) => "Offseason program, phase 2",
            (Block::Otas, _) => "OTAs and minicamp",
            (Block::Break, _) => "Summer break (on their own)",
        }
    }

    pub fn parse(s: &str) -> Option<Block> {
        Block::ALL.iter().copied().find(|b| b.name() == s)
    }

    /// Whether the staff run nothing: the players train on their own.
    pub fn own(self) -> bool {
        matches!(self, Block::Quiet | Block::Break)
    }
}

/// The dates that place a team's blocks in one football year.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    pub tier: Tier,
    pub season: i32,
    /// The football year: from the day after one Super Bowl to the next.
    pub start: Date,
    pub end: Date,
    pub camp: Date,
    pub first_game: Date,
    pub season_end: Date,
    /// Spring practice (none at Division III, or in a state without
    /// spring ball).
    pub spring: Option<(Date, Date)>,
    pub summer: Date,
    /// College: the second semester opens and winter conditioning starts.
    pub winter: Date,
    /// NFL: the offseason program's first day (two weeks earlier for a new
    /// head coach), OTAs, the minicamp's last day and report day.
    pub phase1: Date,
    pub otas: Date,
    pub minicamp_end: Date,
    pub report: Date,
    /// An FBS program or an NFL club: the staff and money for the newer
    /// tools (VR, vision boards, a sports psychologist; `plan::available`).
    #[serde(default)]
    pub big: bool,
}

impl Frame {
    /// A college team's year. `division_end` is its last regular-season
    /// Saturday; `spring` false at Division III, which has no spring
    /// practice.
    pub fn college(y: &YearDates, division_end: Date, spring: bool) -> Frame {
        let first = y.cfb_weeks.first().copied().unwrap_or(y.labor_day);
        Frame {
            tier: Tier::College,
            season: y.season,
            start: y.start,
            end: y.super_bowl,
            camp: y.fall_camp,
            first_game: first,
            season_end: division_end,
            spring: spring.then_some((y.spring_practice.0, y.spring_game)),
            summer: y.summer_workouts.0,
            winter: y.spring_semester,
            phase1: y.start,
            otas: y.start,
            minicamp_end: y.start,
            report: y.fall_camp,
            big: false,
        }
    }

    /// An NFL club's year (a new head coach starts the program two weeks
    /// early, CBA Article 21).
    pub fn pro(y: &YearDates, new_coach: bool) -> Frame {
        let phase1 = if new_coach { y.offseason_new_coach } else { y.offseason_returning };
        Frame {
            tier: Tier::Pro,
            season: y.season,
            start: y.start,
            end: y.super_bowl,
            camp: y.veterans_report,
            first_game: y.nfl_kickoff,
            season_end: y.black_monday.add(-1),
            spring: None,
            summer: y.minicamp.1.add(1),
            winter: y.black_monday,
            phase1,
            otas: y.otas.0,
            minicamp_end: y.minicamp.1,
            report: y.veterans_report,
            big: true,
        }
    }

    /// A high school's year in its state: practice opens three weeks before
    /// the first game, the season runs to the state final, spring ball is
    /// May where the state allows it, and summer is June to camp.
    pub fn high(y: &YearDates, first_game: Date, finals: Date, spring_ball: bool) -> Frame {
        let s = y.season;
        Frame {
            tier: Tier::High,
            season: s,
            start: y.start,
            end: y.super_bowl,
            camp: first_game.add(-21),
            first_game,
            season_end: finals,
            spring: spring_ball.then_some((Date::ymd(s, 5, 1), Date::ymd(s, 5, 31))),
            summer: Date::ymd(s, 6, 1),
            winter: finals.add(1),
            phase1: y.start,
            otas: y.start,
            minicamp_end: y.start,
            report: first_game.add(-21),
            big: false,
        }
    }

    /// The block a day falls in.
    pub fn block(&self, d: Date) -> Block {
        match self.tier {
            Tier::Pro => {
                if d >= self.winter || d < self.phase1 {
                    Block::Winter
                } else if d < self.phase1.add(14) {
                    Block::Phase1
                } else if d < self.otas {
                    Block::Phase2
                } else if d <= self.minicamp_end {
                    Block::Otas
                } else if d < self.report {
                    Block::Break
                } else if d < self.first_game.add(-4) {
                    Block::Camp
                } else {
                    Block::Season
                }
            }
            Tier::College => {
                if d >= self.winter {
                    return Block::Winter;
                }
                if let Some((a, b)) = self.spring {
                    if d >= a && d <= b {
                        return Block::Spring;
                    }
                    if d > b && d < self.summer {
                        return Block::Quiet;
                    }
                }
                if d < self.summer {
                    if self.spring.is_none() && d >= Date::ymd(self.season, 5, 1) {
                        return Block::Quiet;
                    }
                    Block::Winter
                } else if d < self.camp {
                    Block::Summer
                } else if d < self.first_game.add(-5) {
                    Block::Camp
                } else if d <= self.season_end {
                    Block::Season
                } else {
                    Block::Postseason
                }
            }
            Tier::High => {
                if d > self.season_end {
                    return Block::Winter;
                }
                if let Some((a, b)) = self.spring
                    && d >= a
                    && d <= b
                {
                    return Block::Spring;
                }
                if d < self.summer {
                    Block::Winter
                } else if d < self.camp {
                    Block::Summer
                } else if d < self.first_game {
                    Block::Camp
                } else {
                    Block::Season
                }
            }
        }
    }

    /// Mondays in each block of this football year: a block's share of the
    /// year spreads evenly over its weeks.
    pub fn weeks(&self) -> [u8; BLOCKS] {
        let mut n = [0u8; BLOCKS];
        let mut d = self.start.on_or_after(Weekday::Mon);
        while d <= self.end {
            let b = self.block(d) as usize;
            n[b] = n[b].saturating_add(1);
            d = d.add(7);
        }
        n
    }

    /// Days since the block began (the camp's acclimatization days).
    pub fn day_of_block(&self, d: Date) -> i32 {
        let b = self.block(d);
        let mut k = 0;
        let mut x = d;
        while k < 120 && self.block(x.add(-1)) == b {
            x = x.add(-1);
            k += 1;
        }
        k
    }
}

/// The share of a year's growth each block holds, by family (strength,
/// speed, conditioning, technique, contact, kicking, mental). Each column
/// adds to one over the year: the offseason grows the body, spring and OTAs
/// technique and the scheme, camp the battles and contact, the season keeps
/// and prepares (calendar topic 7, confirmed; the split is proposed, not
/// confirmed).
pub fn block_share(tier: Tier, b: Block, spring: bool) -> [f64; FAMILIES] {
    use Block::*;
    let z = [0.0; FAMILIES];
    match tier {
        Tier::College => {
            let base = match b {
                Camp => [0.10, 0.10, 0.15, 0.25, 0.35, 0.20, 0.20],
                Season => [0.12, 0.05, 0.15, 0.30, 0.30, 0.45, 0.45],
                Postseason => [0.03, 0.02, 0.03, 0.05, 0.05, 0.05, 0.05],
                Winter => [0.35, 0.35, 0.25, 0.03, 0.0, 0.10, 0.07],
                Spring => [0.08, 0.08, 0.07, 0.25, 0.30, 0.10, 0.13],
                Quiet => [0.02, 0.02, 0.02, 0.02, 0.0, 0.02, 0.0],
                Summer => [0.30, 0.38, 0.33, 0.10, 0.0, 0.08, 0.10],
                _ => z,
            };
            if spring {
                return base;
            }
            // No spring practice (Division III): the body's spring share is
            // winter's, the football's goes to camp.
            match b {
                Spring => z,
                Winter => [0.43, 0.43, 0.32, 0.03, 0.0, 0.10, 0.07],
                Camp => [0.10, 0.10, 0.15, 0.50, 0.65, 0.30, 0.33],
                _ => base,
            }
        }
        Tier::Pro => match b {
            Camp => [0.10, 0.08, 0.20, 0.30, 0.55, 0.20, 0.20],
            Season => [0.10, 0.04, 0.10, 0.30, 0.40, 0.45, 0.45],
            Winter => [0.40, 0.40, 0.30, 0.08, 0.0, 0.15, 0.05],
            Phase1 => [0.08, 0.08, 0.08, 0.0, 0.0, 0.0, 0.01],
            Phase2 => [0.07, 0.10, 0.07, 0.12, 0.0, 0.05, 0.06],
            Otas => [0.05, 0.05, 0.05, 0.17, 0.05, 0.10, 0.18],
            Break => [0.20, 0.25, 0.20, 0.03, 0.0, 0.05, 0.05],
            _ => z,
        },
        Tier::High => {
            let base = match b {
                Camp => [0.05, 0.03, 0.10, 0.12, 0.15, 0.10, 0.12],
                Season => [0.10, 0.07, 0.15, 0.28, 0.40, 0.30, 0.43],
                Winter => [0.45, 0.35, 0.25, 0.10, 0.0, 0.20, 0.10],
                Spring => [0.05, 0.05, 0.05, 0.20, 0.30, 0.10, 0.15],
                Summer => [0.35, 0.50, 0.45, 0.30, 0.15, 0.30, 0.20],
                _ => z,
            };
            if spring {
                return base;
            }
            match b {
                Spring => z,
                Winter => [0.50, 0.40, 0.30, 0.10, 0.0, 0.20, 0.10],
                Summer => [0.35, 0.50, 0.45, 0.50, 0.45, 0.40, 0.35],
                _ => base,
            }
        }
    }
}

/// The calendar's reach for one week in block `b`: its share spread over
/// the block's weeks.
pub fn weekly_base(f: &Frame, weeks: &[u8; BLOCKS], b: Block) -> [f64; FAMILIES] {
    let share = block_share(f.tier, b, f.spring.is_some());
    let n = weeks[b as usize].max(1) as f64;
    share.map(|s| s / n)
}

/// What the rules of a day allow (topic 3): read by level, block, era and
/// state.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Book {
    pub tier: Tier,
    pub block: Block,
    pub season: i32,
    /// Coaches may run anything at all (false: the players' own time).
    pub coaches: bool,
    /// On-field football with a ball and coaches (false: only the weight
    /// room, conditioning and film where allowed).
    pub field: bool,
    /// Offense against defense (NFL phase 2 has none).
    pub team: bool,
    pub max_gear: Gear,
    pub max_contact: Contact,
    /// Countable minutes a week and a day (NCAA), the most on-field
    /// minutes in a day (NFL, high school heat rules).
    pub week_min: Option<u16>,
    pub day_min: Option<u16>,
    pub field_min: Option<u16>,
    /// Film and meetings a week out of season (NCAA: two hours).
    pub film_week: Option<u16>,
    /// Days a week with nothing required.
    pub days_off: u8,
    /// Most practice days a week (NFL offseason program: four).
    pub practice_days: Option<u8>,
    /// Full contact (thud and live): minutes in one practice, minutes in a
    /// week, days in a week, and whether two days in a row are allowed.
    pub full_session: Option<u16>,
    pub full_week: Option<u16>,
    pub full_days: Option<u8>,
    pub back_to_back: bool,
    /// Two practices on one day.
    pub two_a_days: bool,
    /// The players choose whether to come (NFL phases 1-3 but minicamp).
    pub voluntary: bool,
    /// The rule's source, for the screens.
    pub rule: &'static str,
    /// The team's frame is an FBS program or an NFL club (`Frame::big`).
    #[serde(default)]
    pub big: bool,
}

/// High school contact rules by state: Texas (UIL, 2017: 90 minutes a week
/// of full contact in season), California (AB 2127, 2015: two full-contact
/// practices a week of at most 90 minutes, none out of season); other
/// states follow the NFHS's 2014 guidance (proposed, not confirmed: 90
/// minutes a week).
fn hs_contact(state: &str, season: i32, b: Block) -> (Option<u16>, Option<u16>, Option<u8>, &'static str) {
    let in_season = matches!(b, Block::Camp | Block::Season);
    match state {
        "CA" if season >= 2015 => {
            if in_season {
                (Some(90), Some(180), Some(2), "California AB 2127: two full-contact practices a week, 90 minutes each")
            } else {
                (Some(0), Some(0), Some(0), "California AB 2127: no full contact out of season")
            }
        }
        "TX" if season >= 2017 && b == Block::Season => {
            (None, Some(90), None, "UIL: 90 minutes of full contact a week in season")
        }
        _ if season >= 2014 && b == Block::Season => {
            (None, Some(90), None, "NFHS 2014 guidance: 90 minutes of full contact a week in season")
        }
        _ => (None, None, None, "No state contact limit in this era"),
    }
}

/// The rule book of a day. `day` is the day of the block (camp's
/// acclimatization), `state` a high school's state code.
pub fn book(tier: Tier, b: Block, season: i32, day: i32, state: &str, heat_rule: bool) -> Book {
    let mut k = Book {
        tier,
        block: b,
        season,
        coaches: true,
        field: true,
        team: true,
        max_gear: Gear::Pads,
        max_contact: Contact::Live,
        week_min: None,
        day_min: None,
        field_min: None,
        film_week: None,
        days_off: 0,
        practice_days: None,
        full_session: None,
        full_week: None,
        full_days: None,
        back_to_back: true,
        two_a_days: true,
        voluntary: false,
        rule: "",
        big: tier == Tier::Pro,
    };
    match tier {
        Tier::College => {
            let limits = season >= 1991;
            match b {
                Block::Camp => {
                    k.rule = "NCAA: preseason practice before classes is outside the hour limits";
                }
                Block::Season | Block::Postseason | Block::Spring => {
                    if limits {
                        k.week_min = Some(20 * 60);
                        k.day_min = Some(4 * 60);
                        k.days_off = 1;
                        k.rule = "NCAA: 20 countable hours a week, 4 a day, one day off";
                    } else {
                        k.rule = "NCAA: no hour limits before 1991";
                    }
                }
                Block::Winter => {
                    k.field = false;
                    k.team = false;
                    k.max_gear = Gear::None;
                    k.max_contact = Contact::Air;
                    if limits {
                        k.week_min = Some(8 * 60);
                        k.days_off = 2;
                    }
                    if season >= 2013 {
                        k.film_week = Some(120);
                    }
                    k.rule = "NCAA: out of season, 8 hours a week of strength and conditioning, up to 2 of film";
                }
                Block::Summer => {
                    k.field = false;
                    k.team = false;
                    k.max_gear = Gear::None;
                    k.max_contact = Contact::Air;
                    if season >= 2013 {
                        k.week_min = Some(8 * 60);
                        k.film_week = Some(120);
                        k.days_off = 2;
                        k.rule = "NCAA: summer access, 8 hours a week for 8 weeks, up to 2 of film and walkthrough";
                    } else {
                        k.coaches = false;
                        k.rule = "NCAA: summer workouts are voluntary before 2013";
                        k.voluntary = true;
                    }
                }
                Block::Quiet => {
                    k.coaches = false;
                    k.field = false;
                    k.rule = "NCAA: no required activities";
                }
                _ => {}
            }
            if b == Block::Spring {
                k.rule = "NCAA: 15 spring practices in 34 days, 12 with contact, 3 scrimmages";
                if season >= 2022 {
                    k.back_to_back = false;
                    k.full_session = Some(75);
                    k.rule =
                        "NCAA 2022: 15 spring practices, no full contact on back-to-back days, 75 minutes a session";
                }
                k.two_a_days = false;
            }
            if b == Block::Camp {
                // Acclimatization: helmets only the first two days, then
                // shells, pads after the period (5 days from 2003, 7 from
                // 2021). Two-a-days ended in 2017.
                let accl = if season >= 2021 {
                    7
                } else if season >= 2003 {
                    5
                } else {
                    0
                };
                if day < 2 && accl > 0 {
                    k.max_gear = Gear::Helmets;
                    k.max_contact = Contact::Bags;
                } else if day < accl {
                    k.max_gear = Gear::Shells;
                    k.max_contact = Contact::Control;
                }
                k.two_a_days = season < 2017 && day >= accl;
                if season >= 2021 {
                    k.full_session = Some(75);
                    k.rule = "NCAA 2021: 25 practices in 29 days, 7-day acclimatization, 18 contact, 9 in full pads";
                } else if season >= 2017 {
                    k.rule = "NCAA 2017: 29 camp practices, no two-a-days";
                } else if accl > 0 {
                    k.rule = "NCAA 2003: 5-day acclimatization, two-a-days allowed after it";
                } else {
                    k.rule = "NCAA: two-a-days in full pads";
                }
                if season >= 2021 {
                    k.back_to_back = true;
                }
            }
            if b == Block::Season && season >= 2017 {
                k.two_a_days = false;
            }
        }
        Tier::Pro => {
            let cba = season >= 2011;
            match b {
                Block::Winter | Block::Break => {
                    k.coaches = false;
                    k.field = false;
                    k.team = false;
                    k.rule = "CBA: players' own time";
                }
                Block::Phase1 => {
                    k.voluntary = true;
                    k.practice_days = Some(4);
                    k.day_min = Some(4 * 60);
                    if cba {
                        k.field = false;
                        k.team = false;
                        k.max_gear = Gear::None;
                        k.max_contact = Contact::Air;
                        k.rule = "CBA 2011: phase 1, strength and conditioning and rehab only";
                    } else {
                        k.max_gear = Gear::Helmets;
                        k.max_contact = Contact::Bags;
                        k.rule = "Offseason program before 2011: voluntary, no pads";
                    }
                }
                Block::Phase2 => {
                    k.voluntary = true;
                    k.practice_days = Some(4);
                    k.day_min = Some(4 * 60);
                    k.max_gear = if cba { Gear::None } else { Gear::Helmets };
                    k.max_contact = if cba { Contact::Air } else { Contact::Bags };
                    k.team = !cba;
                    k.rule = if cba {
                        "CBA 2011: phase 2, individual and group drills, no offense against defense, no helmets"
                    } else {
                        "Offseason program before 2011: voluntary, no pads"
                    };
                }
                Block::Otas => {
                    k.voluntary = true;
                    k.practice_days = Some(4);
                    k.day_min = Some(4 * 60);
                    k.max_gear = Gear::Helmets;
                    k.max_contact = if cba { Contact::Air } else { Contact::Bags };
                    k.two_a_days = !cba;
                    if cba {
                        k.field_min = Some(120);
                    }
                    k.rule = if cba {
                        "CBA 2011: 10 OTA days and a 3-day mandatory minicamp, no live contact, no pads"
                    } else {
                        "OTAs before 2011: 14 days, no pads"
                    };
                }
                Block::Camp => {
                    if cba {
                        k.field_min = Some(4 * 60);
                        k.two_a_days = false;
                        if season >= 2020 && day < 5 {
                            k.max_gear = Gear::Helmets;
                            k.max_contact = Contact::Bags;
                        }
                        k.rule = if season >= 2020 {
                            "CBA 2020: 5-day acclimation, then up to 16 padded practices, no more than 3 in a row"
                        } else {
                            "CBA 2011: one padded practice a day, 4 hours on the field"
                        };
                    } else {
                        k.rule = "Camp before 2011: two-a-days in pads";
                    }
                }
                Block::Season => {
                    k.days_off = 1;
                    if cba {
                        k.field_min = Some(3 * 60);
                        k.two_a_days = false;
                        k.rule = "CBA 2011: 14 padded practices in the season, 11 in the first 11 weeks";
                    } else {
                        k.rule = "NFL season before 2011";
                    }
                }
                _ => {}
            }
        }
        Tier::High => {
            let (session, week, days, why) = hs_contact(state, season, b);
            k.full_session = session;
            k.full_week = week;
            k.full_days = days;
            k.rule = why;
            match b {
                Block::Winter => {
                    k.field = false;
                    k.team = false;
                    k.max_gear = Gear::None;
                    k.max_contact = Contact::Air;
                    k.rule = "Offseason: the school's weight program";
                }
                Block::Summer => {
                    k.max_gear = Gear::Helmets;
                    k.max_contact = Contact::Bags;
                    k.voluntary = true;
                    k.rule = "Summer: 7-on-7, camps and the weight program, no pads";
                }
                Block::Spring => {
                    k.max_gear = Gear::Pads;
                }
                Block::Camp => {
                    // Heat acclimatization (NATA 2009, adopted widely from
                    // 2011): helmets the first two days, shells to day five,
                    // pads from day six, three hours a practice, no
                    // two-a-days the first five days.
                    if heat_rule || season >= 2011 {
                        if day < 2 {
                            k.max_gear = Gear::Helmets;
                            k.max_contact = Contact::Bags;
                        } else if day < 5 {
                            k.max_gear = Gear::Shells;
                            k.max_contact = Contact::Control;
                        }
                        k.field_min = Some(180);
                        k.two_a_days = day >= 5;
                    }
                }
                Block::Season => {
                    k.days_off = 1;
                    k.field_min = Some(180);
                    k.two_a_days = false;
                }
                _ => {}
            }
        }
    }
    k
}

/// What breaks a rule, with the rule's words.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Breach {
    /// Day of the week (0 Monday), or None for the week as a whole.
    pub day: Option<u8>,
    pub text: String,
}

fn hm(min: u32) -> String {
    if min.is_multiple_of(60) { format!("{} h", min / 60) } else { format!("{} h {} min", min / 60, min % 60) }
}

/// Checks one day against its book.
pub fn check_day(k: &Book, d: &Day, wd: u8) -> Vec<Breach> {
    let mut out = Vec::new();
    let mut add = |t: String| out.push(Breach { day: Some(wd), text: t });
    if d.periods.is_empty() {
        return out;
    }
    if !k.coaches && d.periods.iter().any(|p| p.kind != Kind::Recovery && !p.players_own()) {
        add(format!("Nothing may be required in this block ({}).", k.rule));
        return out;
    }
    let countable = d.countable();
    if let Some(m) = k.day_min
        && countable > m as u32
    {
        add(format!("{} countable, {} allowed in a day ({}).", hm(countable), hm(m as u32), k.rule));
    }
    if let Some(m) = k.field_min
        && d.field() > m as u32
    {
        add(format!("{} on the field, {} allowed in a day.", hm(d.field()), hm(m as u32)));
    }
    let full: u32 = d.periods.iter().filter(|p| p.contact.full()).map(|p| p.min as u32).sum();
    if let Some(m) = k.full_session
        && full > m as u32
        && !d.periods.iter().any(|p| p.kind == Kind::Scrimmage)
    {
        add(format!("{full} minutes of full contact, {m} allowed in one practice."));
    }
    for p in &d.periods {
        if !k.field && p.kind.on_field() && !p.kind.strength_staff() && !p.own() {
            add(format!("{}: no on-field football in this block ({}).", p.kind.label(), k.rule));
        }
        if !k.team && p.kind.offense_against_defense() {
            add(format!("{}: no offense against defense in this block.", p.kind.label()));
        }
        if p.gear > k.max_gear {
            add(format!("{}: {} not allowed today (at most {}).", p.kind.label(), p.gear.label(), k.max_gear.label()));
        }
        if p.contact > k.max_contact {
            add(format!(
                "{}: {} contact not allowed today (at most {}).",
                p.kind.label(),
                p.contact.label(),
                k.max_contact.label()
            ));
        }
        if p.contact > p.gear.most_contact() {
            add(format!("{}: {} contact needs more than {}.", p.kind.label(), p.contact.label(), p.gear.label()));
        }
    }
    if !k.two_a_days && d.second {
        add("Two practices on one day are not allowed.".to_string());
    }
    out
}

/// Checks a week: each day, then the week's hours, film, days off, full
/// contact and practice days.
pub fn check_week(books: &[Book; 7], w: &Week) -> Vec<Breach> {
    let mut out = Vec::new();
    for (i, d) in w.days.iter().enumerate() {
        out.extend(check_day(&books[i], d, i as u8));
    }
    let k = books.iter().find(|b| b.coaches).unwrap_or(&books[0]);
    let mut add = |t: String| out.push(Breach { day: None, text: t });
    let countable: u32 = w.days.iter().map(|d| d.countable()).sum();
    if let Some(m) = k.week_min
        && countable > m as u32
    {
        add(format!("{} countable this week, {} allowed ({}).", hm(countable), hm(m as u32), k.rule));
    }
    if let Some(m) = k.film_week {
        let film: u32 = w.days.iter().flat_map(|d| &d.periods).filter(|p| p.kind.film()).map(|p| p.min as u32).sum();
        if film > m as u32 {
            add(format!("{} of film and meetings, {} allowed out of season.", hm(film), hm(m as u32)));
        }
    }
    let off = w.days.iter().filter(|d| d.rest()).count() as u8;
    if off < k.days_off {
        add(format!("{} day(s) off needed, {off} planned.", k.days_off));
    }
    if let Some(n) = k.practice_days {
        let days = w.days.iter().filter(|d| !d.rest()).count() as u8;
        if days > n {
            add(format!("{days} days of work, {n} allowed a week."));
        }
    }
    let full: Vec<u32> =
        w.days.iter().map(|d| d.periods.iter().filter(|p| p.contact.full()).map(|p| p.min as u32).sum()).collect();
    if let Some(m) = k.full_week {
        let t: u32 = full.iter().sum();
        if t > m as u32 {
            add(format!("{t} minutes of full contact this week, {m} allowed."));
        }
    }
    if let Some(n) = k.full_days {
        let days = full.iter().filter(|&&m| m > 0).count() as u8;
        if days > n {
            add(format!("{days} full-contact days, {n} allowed a week."));
        }
    }
    if !k.back_to_back && full.windows(2).any(|x| x[0] > 0 && x[1] > 0) {
        add("Full contact on back-to-back days is not allowed.".to_string());
    }
    out
}

/// Counts a whole block must keep (the NCAA's spring and camp practices,
/// the NFL's padded practices), with what has been used so far.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Counts {
    pub practices: u16,
    pub contact: u16,
    pub padded: u16,
    pub scrimmages: u16,
}

/// The block's limits: (practices, contact practices, padded practices,
/// scrimmages). None where the rules set none.
pub fn block_limits(tier: Tier, b: Block, season: i32) -> Counts {
    let none = u16::MAX;
    match (tier, b) {
        (Tier::College, Block::Spring) => Counts { practices: 15, contact: 12, padded: 12, scrimmages: 3 },
        (Tier::College, Block::Camp) if season >= 2021 => {
            Counts { practices: 25, contact: 18, padded: 9, scrimmages: 2 }
        }
        (Tier::College, Block::Camp) => Counts { practices: 29, contact: none, padded: none, scrimmages: none },
        (Tier::College, Block::Postseason) => Counts { practices: 15, contact: none, padded: none, scrimmages: none },
        (Tier::Pro, Block::Otas) if season >= 2011 => Counts { practices: 13, contact: 0, padded: 0, scrimmages: 0 },
        (Tier::Pro, Block::Camp) if season >= 2020 => {
            Counts { practices: none, contact: none, padded: 16, scrimmages: none }
        }
        (Tier::Pro, Block::Season) if season >= 2011 => {
            Counts { practices: none, contact: none, padded: 14, scrimmages: none }
        }
        _ => Counts { practices: none, contact: none, padded: none, scrimmages: none },
    }
}

impl Counts {
    /// No limits at all.
    pub fn unlimited() -> Counts {
        Counts { practices: u16::MAX, contact: u16::MAX, padded: u16::MAX, scrimmages: u16::MAX }
    }

    pub fn add_week(&mut self, w: &Week) {
        for d in &w.days {
            if d.rest() || !d.periods.iter().any(|p| p.practice()) {
                continue;
            }
            self.practices += 1 + d.second as u16;
            if d.periods.iter().any(|p| p.contact >= Contact::Thud) {
                self.contact += 1;
            }
            if d.periods.iter().any(|p| p.gear == Gear::Pads) {
                self.padded += 1;
            }
            if d.periods.iter().any(|p| p.kind == Kind::Scrimmage) {
                self.scrimmages += 1;
            }
        }
    }

    /// What passes a block's limits, in words.
    pub fn over(&self, lim: &Counts) -> Vec<String> {
        let mut v = Vec::new();
        let mut c = |used: u16, max: u16, what: &str| {
            if max != u16::MAX && used > max {
                v.push(format!("{used} {what}, {max} allowed this block."));
            }
        };
        c(self.practices, lim.practices, "practices");
        c(self.contact, lim.contact, "contact practices");
        c(self.padded, lim.padded, "padded practices");
        c(self.scrimmages, lim.scrimmages, "scrimmages");
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_family_adds_to_one_year() {
        for tier in [Tier::High, Tier::College, Tier::Pro] {
            for spring in [true, false] {
                let mut sum = [0.0; FAMILIES];
                for b in Block::ALL {
                    let s = block_share(tier, b, spring);
                    for f in 0..FAMILIES {
                        sum[f] += s[f];
                    }
                }
                for (f, s) in sum.iter().enumerate() {
                    assert!((s - 1.0).abs() < 1e-9, "{tier:?} spring {spring} family {f}: {s}");
                }
            }
        }
    }
}
