//! Calendar data: rivalries with their trophies, the bowls with their tie-ins,
//! each state's high school calendar, and indoor stadiums.
//!
//! Sources: rivalry trophies and bowl tie-ins as they stood in the 2025
//! season (bowl and conference sites, Wikipedia lists of rivalry trophies and
//! bowl games), first years from each bowl's history. High school dates are
//! the 2026 calendars (`data/real-world/hs_states.csv` for playoffs and
//! finals; season starts from the state associations' 2026 calendars, to
//! check). Spring football, grading checks and heat rules by state are
//! researched summaries (to check per state when the network allows).

/// When a rivalry is played.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RivalrySlot {
    /// Rivalry week, the last regular-season Saturday.
    RivalryWeek,
    /// A fixed college week (0 = Week 0).
    Week(usize),
    /// Army-Navy's own Saturday in December.
    ArmyNavy,
}

pub struct Rivalry {
    pub a: &'static str,
    pub b: &'static str,
    pub name: &'static str,
    pub trophy: Option<&'static str>,
    pub slot: RivalrySlot,
    /// A neutral site (state code, town), if played away from both campuses.
    pub neutral: Option<(&'static str, &'static str)>,
}

impl Rivalry {
    /// The game's name in a season: a few names changed over the years.
    /// Oregon and Oregon State's game was the Civil War through 2019 (from
    /// memory, to verify).
    pub fn name_in(&self, season: i32) -> &'static str {
        match (self.a, self.b) {
            ("Oregon", "Oregon State") if season <= 2019 => "the Civil War",
            _ => self.name,
        }
    }
}

macro_rules! r {
    ($a:expr, $b:expr, $n:expr, $t:expr, $s:expr) => {
        Rivalry { a: $a, b: $b, name: $n, trophy: $t, slot: $s, neutral: None }
    };
    ($a:expr, $b:expr, $n:expr, $t:expr, $s:expr, $st:expr, $town:expr) => {
        Rivalry { a: $a, b: $b, name: $n, trophy: $t, slot: $s, neutral: Some(($st, $town)) }
    };
}

use RivalrySlot::*;

/// The rivalries the calendar marks (A28). A program's first listed
/// rivalry-week game is its rivalry-week game.
pub const RIVALRIES: &[Rivalry] = &[
    r!("Ohio State", "Michigan", "The Game", None, RivalryWeek),
    r!("Alabama", "Auburn", "the Iron Bowl", Some("the Foy-ODK Trophy"), RivalryWeek),
    r!("Army", "Navy", "the Army-Navy Game", Some("the Thompson Cup"), ArmyNavy, "PA", "Philadelphia"),
    r!("Georgia", "Georgia Tech", "Clean, Old-Fashioned Hate", Some("the Governor's Cup"), RivalryWeek),
    r!("Clemson", "South Carolina", "the Palmetto Bowl", Some("the Hardee's Trophy"), RivalryWeek),
    r!("Florida", "Florida State", "the Sunshine Showdown", None, RivalryWeek),
    r!("Oregon", "Oregon State", "the Oregon-Oregon State game", Some("the Platypus Trophy"), RivalryWeek),
    r!("Washington", "Washington State", "the Apple Cup", Some("the Apple Cup"), RivalryWeek),
    r!("USC", "UCLA", "the Crosstown Showdown", Some("the Victory Bell"), RivalryWeek),
    r!("Stanford", "California", "the Big Game", Some("the Stanford Axe"), RivalryWeek),
    r!("Texas", "Texas A&M", "the Lone Star Showdown", Some("the State Championship Trophy"), RivalryWeek),
    r!("Ole Miss", "Mississippi State", "the Egg Bowl", Some("the Golden Egg"), RivalryWeek),
    r!("LSU", "Texas A&M", "the LSU-A&M rivalry", None, RivalryWeek),
    r!("Tennessee", "Vanderbilt", "the Tennessee-Vanderbilt rivalry", None, RivalryWeek),
    r!("Minnesota", "Wisconsin", "the Battle for the Axe", Some("Paul Bunyan's Axe"), RivalryWeek),
    r!("Indiana", "Purdue", "the Old Oaken Bucket game", Some("the Old Oaken Bucket"), RivalryWeek),
    r!("Illinois", "Northwestern", "the Land of Lincoln game", Some("the Land of Lincoln Trophy"), RivalryWeek),
    r!("Iowa", "Nebraska", "the Heroes Game", Some("the Heroes Trophy"), RivalryWeek),
    r!("Penn State", "Michigan State", "the Land Grant game", Some("the Land Grant Trophy"), RivalryWeek),
    r!("Maryland", "Rutgers", "the Maryland-Rutgers game", None, RivalryWeek),
    r!("Kansas", "Kansas State", "the Sunflower Showdown", Some("the Governor's Cup"), RivalryWeek),
    r!("Oklahoma", "Oklahoma State", "Bedlam", Some("the Bedlam Bell"), RivalryWeek),
    r!("Utah", "BYU", "the Holy War", Some("the Beehive Boot"), RivalryWeek),
    r!("Arizona", "Arizona State", "the Duel in the Desert", Some("the Territorial Cup"), RivalryWeek),
    r!("Duke", "North Carolina", "the Battle for the Bell", Some("the Victory Bell"), RivalryWeek),
    r!("Virginia", "Virginia Tech", "the Commonwealth Clash", Some("the Commonwealth Cup"), RivalryWeek),
    r!("Louisville", "Kentucky", "the Governor's Cup game", Some("the Governor's Cup"), RivalryWeek),
    r!("Pittsburgh", "West Virginia", "the Backyard Brawl", None, Week(1)),
    r!("Miami (FL)", "Florida State", "the Miami-FSU rivalry", None, Week(6)),
    r!("Texas", "Oklahoma", "the Red River Rivalry", Some("the Golden Hat"), Week(7), "TX", "Dallas"),
    r!("Florida", "Georgia", "the Florida-Georgia game", Some("the Okefenokee Oar"), Week(9), "FL", "Jacksonville"),
    r!("Alabama", "Tennessee", "the Third Saturday in October", None, Week(8)),
    r!("Alabama", "LSU", "the Alabama-LSU rivalry", None, Week(10)),
    r!("Auburn", "Georgia", "the Deep South's Oldest Rivalry", None, Week(6)),
    r!("Michigan", "Michigan State", "the Paul Bunyan game", Some("the Paul Bunyan Trophy"), Week(8)),
    r!("Michigan", "Minnesota", "the Little Brown Jug game", Some("the Little Brown Jug"), Week(5)),
    r!("Iowa", "Iowa State", "the Cy-Hawk game", Some("the Cy-Hawk Trophy"), Week(2)),
    r!("Iowa", "Minnesota", "the Floyd of Rosedale game", Some("Floyd of Rosedale"), Week(9)),
    r!("Wisconsin", "Iowa", "the Heartland game", Some("the Heartland Trophy"), Week(7)),
    r!("Nebraska", "Wisconsin", "the Freedom Trophy game", Some("the Freedom Trophy"), Week(10)),
    r!("Purdue", "Illinois", "the Purdue Cannon game", Some("the Purdue Cannon"), Week(6)),
    r!("USC", "Notre Dame", "the Jeweled Shillelagh game", Some("the Jeweled Shillelagh"), RivalryWeek),
    r!("Notre Dame", "Navy", "the Notre Dame-Navy game", Some("the Rip Miller Trophy"), Week(10)),
    // Legs of the Commander-in-Chief's Trophy (see `COMMANDER_IN_CHIEF`).
    r!("Navy", "Air Force", "the Navy-Air Force game", None, Week(6)),
    r!("Army", "Air Force", "the Army-Air Force game", None, Week(10)),
    r!("Colorado", "Colorado State", "the Rocky Mountain Showdown", Some("the Centennial Cup"), Week(2)),
    r!("Wyoming", "Colorado State", "the Border War", Some("the Bronze Boot"), RivalryWeek),
    r!("Boise State", "Fresno State", "the Milk Can game", Some("the Milk Can"), Week(9)),
    r!("Nevada", "UNLV", "the Battle for Nevada", Some("the Fremont Cannon"), RivalryWeek),
    r!("New Mexico", "New Mexico State", "the Rio Grande Rivalry", Some("the Maloof Trophy"), Week(3)),
    r!("Toledo", "Bowling Green", "the Battle of I-75", Some("the Battle of I-75 Trophy"), RivalryWeek),
    r!("Ohio", "Miami (OH)", "the Battle of the Bricks", Some("the Bell"), Week(8)),
    r!("Central Michigan", "Western Michigan", "the Michigan MAC Trophy game", Some("the Victory Cannon"), Week(9)),
    r!("Houston", "Rice", "the Bayou Bucket", Some("the Bayou Bucket"), Week(3)),
    r!("SMU", "TCU", "the Battle for the Iron Skillet", Some("the Iron Skillet"), Week(3)),
    r!("Baylor", "TCU", "the Revivalry", None, RivalryWeek),
    r!("Texas Tech", "TCU", "the West Texas Championship", Some("the Saddle Trophy"), Week(10)),
    r!("Utah State", "BYU", "the Old Wagon Wheel game", Some("the Old Wagon Wheel"), Week(5)),
    r!("Southern Miss", "Louisiana Tech", "the Rivalry in Dixie", None, Week(8)),
    r!("Marshall", "Ohio", "the Battle for the Bell", Some("the Bell"), Week(2)),
    r!("UCF", "South Florida", "the War on I-4", None, RivalryWeek),
    r!("Appalachian State", "Georgia Southern", "Deeper Than Hate", None, Week(9)),
    r!("Cincinnati", "Louisville", "the Keg of Nails game", Some("the Keg of Nails"), Week(4)),
    r!("Memphis", "UAB", "the Battle for the Bones", None, Week(9)),
    r!("Kentucky", "Tennessee", "the Battle for the Barrel", None, Week(11)),
    r!("Missouri", "Arkansas", "the Battle Line Rivalry", Some("the Battle Line Trophy"), RivalryWeek),
    r!("LSU", "Arkansas", "the Battle for the Golden Boot", Some("the Golden Boot"), Week(11)),
    r!("Hawaii", "Wyoming", "the Paniolo Trophy game", Some("the Paniolo Trophy"), Week(11)),
    r!("Georgia State", "Georgia Southern", "the Modern Day Hate", None, RivalryWeek),
    r!("Montana", "Montana State", "the Brawl of the Wild", Some("the Great Divide Trophy"), RivalryWeek),
    r!("Harvard", "Yale", "The Game", None, RivalryWeek),
    r!("Lafayette", "Lehigh", "The Rivalry", None, RivalryWeek),
    r!("Grambling State", "Southern", "the Bayou Classic", None, RivalryWeek, "LA", "New Orleans"),
    r!("Florida A&M", "Bethune-Cookman", "the Florida Classic", None, RivalryWeek, "FL", "Orlando"),
    r!("Jackson State", "Alcorn State", "the Soul Bowl", None, RivalryWeek),
    r!("William & Mary", "Richmond", "the Capital Cup game", Some("the Capital Cup"), RivalryWeek),
    r!("North Dakota", "South Dakota", "the Sitting Bull Trophy game", None, Week(9)),
    r!("Eastern Washington", "Idaho", "the Eastern Washington-Idaho game", None, Week(5)),
];

/// The Commander-in-Chief's Trophy and the service academies that play for
/// it (A28): one trophy for their three games with each other, Army-Navy
/// included. It goes to the academy with the best record in them, settled
/// after the Army-Navy game, and stays with its holder when all three split
/// 1-1 (from memory, to verify).
pub const COMMANDER_IN_CHIEF: (&str, [&str; 3]) = ("the Commander-in-Chief's Trophy", ["Army", "Navy", "Air Force"]);

/// A bowl game. `ties` are the conferences it takes, in order of choice;
/// "any" fills from the best eligible team left.
pub struct Bowl {
    pub name: &'static str,
    pub state: &'static str,
    pub town: &'static str,
    pub indoor: bool,
    pub first: i32,
    pub ties: [&'static str; 2],
    /// One of the New Year's Six (a major bowl before 2014: the Rose, Sugar,
    /// Orange and Fiesta; the Cotton and Peach joined the majors in 2014).
    pub major: bool,
    /// Day offset from the start of bowl season (the Saturday after the
    /// conference championships, about Dec 13), or None for a New Year's game.
    pub day: Option<i32>,
}

macro_rules! b {
    ($n:expr, $s:expr, $t:expr, $in:expr, $f:expr, $a:expr, $b:expr, $m:expr, $d:expr) => {
        Bowl { name: $n, state: $s, town: $t, indoor: $in, first: $f, ties: [$a, $b], major: $m, day: $d }
    };
}

pub const BOWLS: &[Bowl] = &[
    b!("Rose Bowl", "CA", "Pasadena", false, 1902, "Big Ten", "Pac-12", true, None),
    b!("Sugar Bowl", "LA", "New Orleans", true, 1935, "SEC", "Big 12", true, None),
    b!("Orange Bowl", "FL", "Miami Gardens", false, 1935, "ACC", "any", true, None),
    b!("Fiesta Bowl", "AZ", "Glendale", true, 1971, "Big 12", "any", true, None),
    b!("Cotton Bowl", "TX", "Arlington", true, 1937, "SEC", "Big 12", true, None),
    b!("Peach Bowl", "GA", "Atlanta", true, 1968, "ACC", "SEC", true, None),
    b!("Citrus Bowl", "FL", "Orlando", false, 1947, "SEC", "Big Ten", false, Some(19)),
    b!("ReliaQuest Bowl", "FL", "Tampa", false, 1986, "SEC", "Big Ten", false, Some(19)),
    b!("Gator Bowl", "FL", "Jacksonville", false, 1946, "SEC", "ACC", false, Some(18)),
    b!("Music City Bowl", "TN", "Nashville", false, 1998, "SEC", "Big Ten", false, Some(17)),
    b!("Texas Bowl", "TX", "Houston", true, 2006, "SEC", "Big 12", false, Some(14)),
    b!("Liberty Bowl", "TN", "Memphis", false, 1959, "SEC", "Big 12", false, Some(14)),
    b!("Las Vegas Bowl", "NV", "Las Vegas", true, 1992, "SEC", "Big Ten", false, Some(14)),
    b!("Holiday Bowl", "CA", "San Diego", false, 1978, "ACC", "Big Ten", false, Some(14)),
    b!("Alamo Bowl", "TX", "San Antonio", true, 1993, "Big 12", "Big Ten", false, Some(17)),
    b!("Pop-Tarts Bowl", "FL", "Orlando", false, 1990, "ACC", "Big 12", false, Some(15)),
    b!("Pinstripe Bowl", "NY", "Bronx", false, 2010, "ACC", "Big Ten", false, Some(14)),
    b!("Duke's Mayo Bowl", "NC", "Charlotte", false, 2002, "ACC", "SEC", false, Some(16)),
    b!("Sun Bowl", "TX", "El Paso", false, 1935, "ACC", "Big 12", false, Some(17)),
    b!("Military Bowl", "MD", "Annapolis", false, 2008, "ACC", "American", false, Some(14)),
    b!("Fenway Bowl", "MA", "Boston", false, 2024, "ACC", "American", false, Some(14)),
    b!("Birmingham Bowl", "AL", "Birmingham", false, 2006, "SEC", "American", false, Some(14)),
    b!("Gasparilla Bowl", "FL", "Tampa", false, 2008, "American", "ACC", false, Some(8)),
    b!("Armed Forces Bowl", "TX", "Fort Worth", false, 2003, "American", "Big 12", false, Some(14)),
    b!("First Responder Bowl", "TX", "Dallas", false, 2010, "Big 12", "CUSA", false, Some(10)),
    b!("Independence Bowl", "LA", "Shreveport", false, 1976, "CUSA", "Pac-12", false, Some(12)),
    b!("Rate Bowl", "AZ", "Phoenix", true, 1989, "Big 12", "Big Ten", false, Some(13)),
    b!("Cure Bowl", "FL", "Orlando", false, 2015, "American", "Sun Belt", false, Some(0)),
    b!("Frisco Bowl", "TX", "Frisco", false, 2017, "American", "Mountain West", false, Some(4)),
    b!("Myrtle Beach Bowl", "SC", "Conway", false, 2020, "Sun Belt", "CUSA", false, Some(3)),
    b!("Boca Raton Bowl", "FL", "Boca Raton", false, 2014, "American", "CUSA", false, Some(5)),
    b!("New Mexico Bowl", "NM", "Albuquerque", false, 2006, "Mountain West", "CUSA", false, Some(1)),
    b!("LA Bowl", "CA", "Inglewood", true, 2021, "Mountain West", "Pac-12", false, Some(0)),
    b!("Hawaii Bowl", "HI", "Honolulu", false, 2002, "Mountain West", "American", false, Some(11)),
    b!("Famous Idaho Potato Bowl", "ID", "Boise", false, 1997, "Mountain West", "MAC", false, Some(10)),
    b!("Arizona Bowl", "AZ", "Tucson", false, 2015, "Mountain West", "MAC", false, Some(15)),
    b!("68 Ventures Bowl", "AL", "Mobile", false, 1999, "MAC", "Sun Belt", false, Some(10)),
    b!("Bahamas Bowl", "FL", "Nassau", false, 2014, "MAC", "CUSA", false, Some(4)),
    b!("Camellia Bowl", "AL", "Montgomery", false, 2014, "MAC", "Sun Belt", false, Some(1)),
    b!("New Orleans Bowl", "LA", "New Orleans", true, 2001, "Sun Belt", "CUSA", false, Some(4)),
    b!("GameAbove Sports Bowl", "MI", "Detroit", true, 1997, "MAC", "Big Ten", false, Some(13)),
    b!("Salute to Veterans Bowl", "AL", "Montgomery", false, 2024, "Sun Belt", "American", false, Some(2)),
];

/// The FCS's HBCU bowl: SWAC against MEAC champions (since 2015).
pub const CELEBRATION_BOWL: (&str, &str, i32) = ("Celebration Bowl", "Atlanta", 2015);

/// How a state checks grades for eligibility (A29).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradeCheck {
    /// Texas "no pass, no play": a failing grade at a six-week report card
    /// sits him at least three weeks.
    SixWeek,
    /// Weekly checks (Pennsylvania, Illinois): a failing week sits him the next.
    Weekly,
    /// A grading period of about nine weeks (Ohio, California).
    NineWeek,
    /// Last semester's grades set eligibility for this one.
    Semester,
}

/// One state's 2026 high school calendar.
pub struct HsCalendar {
    pub state: &'static str,
    /// First regular-season game (month, day) in 2026.
    pub start: (u32, u32),
    pub games: u32,
    /// Playoffs start and finals (month, day) in 2026.
    pub playoffs: (u32, u32),
    pub finals: (u32, u32),
    pub spring_ball: bool,
    pub grades: GradeCheck,
    /// Practice limited by heat index or WBGT (A36).
    pub heat_rule: bool,
}

macro_rules! hs {
    ($s:expr, $sm:expr, $sd:expr, $g:expr, $pm:expr, $pd:expr, $fm:expr, $fd:expr, $spring:expr, $gr:expr, $heat:expr) => {
        HsCalendar {
            state: $s,
            start: ($sm, $sd),
            games: $g,
            playoffs: ($pm, $pd),
            finals: ($fm, $fd),
            spring_ball: $spring,
            grades: $gr,
            heat_rule: $heat,
        }
    };
}

use GradeCheck::*;

pub const HS_CALENDARS: [HsCalendar; 51] = [
    hs!("AL", 8, 21, 10, 11, 6, 12, 4, true, Semester, true),
    hs!("AK", 8, 7, 7, 10, 9, 10, 23, false, Semester, false),
    hs!("AZ", 8, 21, 10, 11, 6, 12, 5, false, Semester, false),
    hs!("AR", 8, 28, 10, 11, 13, 12, 12, true, Semester, true),
    hs!("CA", 8, 21, 10, 11, 6, 12, 12, false, NineWeek, false),
    hs!("CO", 8, 28, 9, 11, 6, 12, 5, false, Semester, false),
    hs!("CT", 9, 11, 10, 12, 1, 12, 12, false, Semester, false),
    hs!("DE", 9, 4, 10, 11, 20, 12, 5, false, Semester, false),
    hs!("DC", 8, 28, 10, 11, 13, 12, 5, false, Semester, false),
    hs!("FL", 8, 21, 10, 11, 13, 12, 12, true, Semester, true),
    hs!("GA", 8, 14, 10, 11, 13, 12, 16, true, Semester, true),
    hs!("HI", 8, 7, 9, 11, 13, 11, 28, false, Semester, false),
    hs!("ID", 8, 28, 9, 10, 30, 11, 21, false, Semester, false),
    hs!("IL", 8, 28, 9, 10, 23, 11, 28, false, Weekly, false),
    hs!("IN", 8, 21, 9, 10, 23, 11, 28, false, Semester, false),
    hs!("IA", 8, 21, 8, 10, 23, 11, 20, false, Semester, false),
    hs!("KS", 9, 4, 8, 10, 30, 11, 28, false, Semester, false),
    hs!("KY", 8, 21, 10, 11, 6, 12, 5, false, Semester, true),
    hs!("LA", 9, 4, 10, 11, 13, 12, 12, true, Semester, true),
    hs!("ME", 9, 4, 8, 10, 30, 11, 21, false, Semester, false),
    hs!("MD", 8, 28, 10, 11, 6, 12, 5, false, Semester, true),
    hs!("MA", 9, 11, 8, 11, 6, 12, 5, false, Semester, false),
    hs!("MI", 8, 27, 9, 10, 30, 11, 28, false, Semester, false),
    hs!("MN", 8, 27, 8, 10, 23, 11, 28, false, Semester, false),
    hs!("MS", 8, 21, 10, 11, 6, 12, 5, true, Semester, true),
    hs!("MO", 8, 21, 9, 10, 30, 12, 5, false, Semester, false),
    hs!("MT", 8, 28, 8, 10, 30, 11, 21, false, Semester, false),
    hs!("NE", 8, 28, 8, 10, 30, 11, 21, false, Semester, false),
    hs!("NV", 8, 21, 9, 10, 30, 11, 21, false, Semester, false),
    hs!("NH", 9, 4, 8, 11, 6, 11, 28, false, Semester, false),
    hs!("NJ", 8, 28, 8, 10, 30, 11, 29, false, Semester, true),
    hs!("NM", 8, 21, 10, 10, 30, 11, 28, false, NineWeek, false),
    hs!("NY", 9, 4, 7, 10, 23, 12, 5, false, Semester, false),
    hs!("NC", 8, 21, 11, 11, 6, 12, 12, false, Semester, true),
    hs!("ND", 8, 21, 8, 10, 24, 11, 13, false, Semester, false),
    hs!("OH", 8, 21, 10, 10, 30, 12, 5, false, NineWeek, false),
    hs!("OK", 8, 28, 10, 11, 13, 12, 12, true, Semester, true),
    hs!("OR", 8, 28, 9, 11, 6, 11, 28, false, Semester, false),
    hs!("PA", 8, 21, 10, 11, 6, 12, 12, false, Weekly, false),
    hs!("RI", 9, 11, 8, 11, 6, 11, 21, false, Semester, false),
    hs!("SC", 8, 21, 10, 11, 6, 12, 5, true, Semester, true),
    hs!("SD", 8, 21, 8, 10, 23, 11, 13, false, Semester, false),
    hs!("TN", 8, 21, 10, 11, 6, 12, 5, true, Semester, true),
    hs!("TX", 8, 28, 10, 11, 13, 12, 18, true, SixWeek, true),
    hs!("UT", 8, 14, 10, 10, 23, 11, 20, false, Semester, false),
    hs!("VT", 9, 4, 8, 10, 30, 11, 14, false, Semester, false),
    hs!("VA", 8, 28, 10, 11, 13, 12, 12, false, Semester, true),
    hs!("WA", 8, 28, 9, 11, 6, 12, 5, false, Semester, false),
    hs!("WV", 8, 28, 10, 11, 13, 12, 5, false, Semester, false),
    hs!("WI", 8, 21, 9, 10, 16, 11, 20, false, Semester, false),
    hs!("WY", 8, 28, 8, 10, 30, 11, 14, false, Semester, false),
];

pub fn hs_calendar(code: &str) -> &'static HsCalendar {
    HS_CALENDARS.iter().find(|c| c.state == code).unwrap_or(&HS_CALENDARS[43])
}

/// College teams that play indoors.
pub const INDOOR_PROGRAMS: [&str; 9] = [
    "Syracuse",
    "UTSA",
    "Idaho",
    "Northern Arizona",
    "North Dakota",
    "North Dakota State",
    "South Dakota",
    "Idaho State",
    "Northern Iowa",
];

/// FBS conference media days (A26), in the order they are usually held in July.
pub const MEDIA_DAYS: [&str; 10] =
    ["Sun Belt", "CUSA", "Mountain West", "SEC", "Big 12", "American", "Big Ten", "ACC", "MAC", "Pac-12"];

/// SAT test dates (A30): (month, nth Saturday). The ACT is held on its own
/// Saturdays; the world schedules both.
pub const SAT_DATES: [(u32, u32); 7] = [(3, 2), (5, 1), (6, 1), (8, 4), (10, 1), (11, 1), (12, 1)];
pub const ACT_DATES: [(u32, u32); 7] = [(2, 2), (4, 2), (6, 2), (7, 2), (9, 2), (10, 4), (12, 2)];

/// Hours a state's football towns sit behind Eastern time during the season
/// (Arizona keeps standard time, so it plays on Pacific clocks in the fall).
pub fn et_offset(code: &str) -> i32 {
    match code {
        "AL" | "AR" | "IL" | "IA" | "KS" | "LA" | "MN" | "MS" | "MO" | "NE" | "ND" | "OK" | "SD" | "TX" | "WI" => 1,
        "CO" | "ID" | "MT" | "NM" | "UT" | "WY" => 2,
        "AZ" | "CA" | "NV" | "OR" | "WA" => 3,
        "AK" => 4,
        "HI" => 6,
        _ => 0,
    }
}
