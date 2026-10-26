//! Who carries a game (update 2, lane 6, SC1): the network by level, era
//! and window, as the schedule page shows it. College follows each
//! conference's rights holders in their years (CBS's SEC game of the week
//! 1996-2023, then ABC; the Big Ten on FOX, CBS and NBC from 2023; the
//! conference networks from their launches: the Big Ten Network 2007, the
//! Pac-12 Network 2012-2024, the SEC Network 2014, the ACC Network 2019);
//! the NFL follows its contracts (CBS the AFC and FOX the NFC from 1994,
//! NBC's Sunday night from 2006, ESPN's Monday night from 2006, Thursday
//! night's carriers by year); high school is the town's radio station and,
//! from 2014, a stream. Proposed, not confirmed: which game of a window
//! goes to which network (the bigger game to the bigger network, by the
//! game's draw), and the networks of the years before 1996.

use super::data::Division;
use super::schedule::GameKind;
use crate::calendar::Date;
use crate::calendar::Weekday;

/// A college game as the networks see it.
pub struct CollegeGame<'a> {
    pub year: i32,
    pub date: Date,
    /// Kickoff on Eastern clocks (hour, minute).
    pub kick: (u16, u16),
    pub kind: GameKind,
    /// The bowl or playoff game's name, if any.
    pub name: Option<&'a str>,
    pub home_conf: &'a str,
    pub away_conf: &'a str,
    pub home_div: Division,
    pub away_div: Division,
    /// The home program's name (Notre Dame's NBC, the Longhorn Network).
    pub home: &'a str,
    pub away: &'a str,
    /// How big the game is, 0-1: the two programs' names and ranks, a
    /// rivalry. The biggest games of a window go to its biggest network.
    pub draw: f64,
    /// 0-1, fixed for the game: which of a window's networks it lands on.
    pub key: f64,
}

/// The network or stream that carries a college game.
pub fn college(g: &CollegeGame) -> &'static str {
    let y = g.year;
    let fbs = |d: Division| d == Division::FBS;
    let fcs = |d: Division| d == Division::FCS;
    // Below Division I: the school's own stream, local radio before.
    if !fbs(g.home_div) && !fcs(g.home_div) && !fbs(g.away_div) {
        return match g.kind {
            GameKind::Title if y >= 2010 => "ESPNU",
            _ if y >= 2012 => "School stream",
            _ => "Local radio",
        };
    }
    match g.kind {
        GameKind::Title => return title(g),
        GameKind::Playoff(round) => return playoff(g, round),
        GameKind::Bowl(_) => return bowl(g),
        GameKind::Celebration => return "ABC",
        GameKind::ConferenceTitle => return conference_title(g.home_conf, y),
        GameKind::Regular => {}
    }
    // An FCS home game: the conference's stream, regional TV before.
    if fcs(g.home_div) && !fbs(g.away_div) {
        let classic = g.home_conf == "SWAC" || g.away_conf == "SWAC" || g.home_conf == "MEAC";
        if classic && g.draw > 0.55 && y >= 2005 {
            return if g.key < 0.5 { "ESPNU" } else { "ESPN2" };
        }
        return match y {
            2019.. => "ESPN+",
            2010..=2018 => "ESPN3",
            _ => "Regional TV",
        };
    }
    let wd = g.date.weekday();
    let (h, _) = g.kick;
    // Army-Navy, on CBS since 1996.
    if (g.home == "Army" && g.away == "Navy") || (g.home == "Navy" && g.away == "Army") {
        return if y >= 1996 { "CBS" } else { "ABC" };
    }
    // Notre Dame's home games, on NBC since 1991.
    if g.home == "Notre Dame" && y >= 1991 {
        return "NBC";
    }
    // Weeknights: ESPN's Thursday nights, Friday nights, Labor Day and the
    // MAC's midweek November games.
    if wd != Weekday::Sat {
        return weeknight(g, wd);
    }
    // An FCS visitor at an FBS school: the conference's own channel or stream.
    let visitor_small = !fbs(g.away_div);
    let conf = if fbs(g.home_div) { g.home_conf } else { g.away_conf };
    let window = match h {
        0..=13 => 0,
        14..=17 => 1,
        18..=21 => 2,
        _ => 3,
    };
    let big = g.draw;
    let k = g.key;
    match conf {
        "SEC" => {
            if visitor_small {
                return match y {
                    2014.. => "SEC Network+",
                    2009..=2013 => "SEC Network",
                    _ => "Regional TV",
                };
            }
            if y >= 2024 {
                return match window {
                    1 if big > 0.6 => "ABC",
                    2 if big > 0.62 => {
                        if k < 0.5 {
                            "ABC"
                        } else {
                            "ESPN"
                        }
                    }
                    0 if big > 0.7 => "ABC",
                    _ if big > 0.45 => {
                        if k < 0.6 {
                            "ESPN"
                        } else {
                            "ESPN2"
                        }
                    }
                    _ => "SEC Network",
                };
            }
            if window == 1 && big > 0.6 && y >= 1996 {
                return "CBS";
            }
            if big > 0.5 {
                return if k < 0.5 { "ESPN" } else { "ESPN2" };
            }
            match y {
                2014.. => "SEC Network",
                2005..=2013 => "ESPNU",
                _ => "Regional TV",
            }
        }
        "Big Ten" => {
            if y >= 2023 {
                if visitor_small {
                    return if k < 0.6 { "Big Ten Network" } else { "Peacock" };
                }
                return match window {
                    0 if big > 0.55 => "FOX",
                    1 if big > 0.55 => "CBS",
                    2 if big > 0.55 => "NBC",
                    _ if big > 0.4 => {
                        if k < 0.5 {
                            "FS1"
                        } else {
                            "Big Ten Network"
                        }
                    }
                    _ => {
                        if k < 0.75 {
                            "Big Ten Network"
                        } else {
                            "Peacock"
                        }
                    }
                };
            }
            if big > 0.6 && !visitor_small {
                return match (y, window) {
                    (2017.., 0) => "FOX",
                    (2017.., _) if k < 0.5 => "FOX",
                    _ => "ABC",
                };
            }
            if y >= 2007 {
                return if big > 0.45 && !visitor_small { "ESPN" } else { "Big Ten Network" };
            }
            if k < 0.5 { "ESPN" } else { "ESPN2" }
        }
        "ACC" => {
            if visitor_small {
                return match y {
                    2019.. => "ACC Network Extra",
                    _ => "Regional TV",
                };
            }
            if big > 0.62 && window >= 1 {
                return "ABC";
            }
            if big > 0.45 {
                return if k < 0.55 { "ESPN" } else { "ESPN2" };
            }
            match y {
                2023.. if window == 0 && k < 0.35 => "The CW",
                2019.. => "ACC Network",
                1982..=2018 => "Raycom",
                _ => "Regional TV",
            }
        }
        "Big 12" => {
            if visitor_small {
                return if y >= 2017 { "ESPN+" } else { "Regional TV" };
            }
            if g.home == "Texas" && (2011..=2023).contains(&y) && big < 0.5 {
                return "Longhorn Network";
            }
            if big > 0.6 {
                return if k < 0.5 { "ABC" } else { "FOX" };
            }
            if big > 0.42 {
                return if k < 0.5 { "ESPN" } else { "FS1" };
            }
            if y >= 2017 { if k < 0.5 { "ESPN+" } else { "ESPN2" } } else { "FSN" }
        }
        "Pac-12" if y >= 2026 => mountain_tv(y, big, k, visitor_small),
        "Pac-12" => {
            if big > 0.6 {
                return if k < 0.5 { "ABC" } else { "FOX" };
            }
            if big > 0.42 {
                return if k < 0.5 { "ESPN" } else { "FS1" };
            }
            if y >= 2012 { "Pac-12 Network" } else { "FSN" }
        }
        "Mountain West" => mountain_tv(y, big, k, visitor_small),
        "American" => {
            if visitor_small || big < 0.3 {
                return if y >= 2020 { "ESPN+" } else { "ESPN3" };
            }
            if big > 0.6 {
                return "ABC";
            }
            if k < 0.5 { "ESPN2" } else { "ESPNU" }
        }
        "Sun Belt" => {
            if visitor_small || big < 0.35 {
                return if y >= 2019 { "ESPN+" } else { "ESPN3" };
            }
            if k < 0.5 { "ESPNU" } else { "ESPN2" }
        }
        "MAC" => {
            if visitor_small || big < 0.35 {
                return if y >= 2019 { "ESPN+" } else { "ESPN3" };
            }
            if k < 0.4 { "CBS Sports Network" } else { "ESPNU" }
        }
        "CUSA" => {
            if y >= 2023 {
                if k < 0.55 { "CBS Sports Network" } else { "ESPN+" }
            } else if k < 0.5 {
                "CBS Sports Network"
            } else {
                "ESPN3"
            }
        }
        // UConn and the other independents.
        _ => {
            if big > 0.55 && y >= 1996 {
                "ESPN"
            } else if y >= 2010 {
                "CBS Sports Network"
            } else {
                "Regional TV"
            }
        }
    }
}

/// The rebuilt Pac-12 and the Mountain West from 2026: CBS and its cable
/// channel, FOX for the Mountain West's biggest, The CW (proposed, not
/// confirmed from their 2026 deals).
fn mountain_tv(y: i32, big: f64, k: f64, small: bool) -> &'static str {
    if small {
        return if y >= 2026 { "The CW" } else { "Mountain West Network" };
    }
    if big > 0.55 {
        return if y >= 2026 || k < 0.5 { "CBS" } else { "FOX" };
    }
    if y >= 2026 {
        if k < 0.6 { "CBS Sports Network" } else { "The CW" }
    } else if k < 0.5 {
        "CBS Sports Network"
    } else {
        "FS1"
    }
}

fn weeknight(g: &CollegeGame, wd: Weekday) -> &'static str {
    let y = g.year;
    match wd {
        Weekday::Thu => {
            if g.home_conf == "Big Ten" && y >= 2023 {
                return "FS1";
            }
            if g.home_conf == "MAC" {
                return if g.key < 0.5 { "ESPNU" } else { "CBS Sports Network" };
            }
            if g.draw > 0.5 || y < 2005 { "ESPN" } else { "ESPNU" }
        }
        Weekday::Fri => {
            if g.home_conf == "Big Ten" && y >= 2023 {
                return if g.draw > 0.55 { "FOX" } else { "FS1" };
            }
            if g.home_conf == "Mountain West" || g.home_conf == "Pac-12" {
                return "CBS Sports Network";
            }
            if g.draw > 0.5 { "ESPN" } else { "ESPN2" }
        }
        // Labor Day night.
        Weekday::Mon => "ESPN",
        // The MAC's midweek nights.
        Weekday::Tue | Weekday::Wed => {
            if g.key < 0.5 {
                "ESPN2"
            } else {
                "ESPNU"
            }
        }
        // Thanksgiving week and the odd Sunday.
        _ => {
            if g.draw > 0.5 {
                "ABC"
            } else {
                "ESPN"
            }
        }
    }
}

/// The conference title games' carriers by year.
fn conference_title(conf: &str, y: i32) -> &'static str {
    match conf {
        "SEC" => {
            if y >= 2024 {
                "ABC"
            } else if y >= 1992 {
                "CBS"
            } else {
                "ABC"
            }
        }
        // FOX, CBS and NBC in turn from 2023.
        "Big Ten" => match y {
            2023.. => ["FOX", "CBS", "NBC"][((y - 2023) % 3) as usize],
            2017..=2022 => "FOX",
            _ => "ABC",
        },
        "Big 12" => {
            if y % 2 == 0 {
                "FOX"
            } else {
                "ABC"
            }
        }
        "ACC" => "ABC",
        "Pac-12" if y >= 2026 => "CBS",
        "Pac-12" => {
            if y % 2 == 0 {
                "ABC"
            } else {
                "FOX"
            }
        }
        "American" => "ABC",
        "Mountain West" => {
            if y >= 2026 {
                "CBS"
            } else {
                "FOX"
            }
        }
        "MAC" | "Sun Belt" => "ESPN",
        "CUSA" => "CBS Sports Network",
        _ => "ESPN",
    }
}

/// Bowls by their carriers (2025): CBS's Sun Bowl, FOX's Holiday Bowl from
/// 2022, ABC's Rose and Citrus, ESPN and its channels for the rest.
fn bowl(g: &CollegeGame) -> &'static str {
    let name = g.name.unwrap_or("");
    let y = g.year;
    // ABC through 2010, ESPN's from the BCS's last deal on.
    if name.contains("Rose") {
        return if y <= 2010 { "ABC" } else { "ESPN" };
    }
    if name.contains("Sun Bowl") {
        return "CBS";
    }
    if name.contains("Holiday") {
        return if y >= 2022 { "FOX" } else { "ESPN" };
    }
    if name.contains("Citrus") || name.contains("Pinstripe") {
        return "ABC";
    }
    if name.contains("Arizona") && y >= 2024 {
        return "The CW";
    }
    if name.contains("Cotton") {
        return if (1999..=2014).contains(&y) { "FOX" } else { "ESPN" };
    }
    // The BCS bowls: ABC to 2006, FOX 2007-2010, ESPN after.
    if name.contains("Sugar") || name.contains("Orange") || name.contains("Fiesta") {
        return match y {
            ..=2006 => "ABC",
            2007..=2010 => "FOX",
            _ => "ESPN",
        };
    }
    if name.contains("Peach") {
        return "ESPN";
    }
    if g.draw > 0.5 {
        "ESPN"
    } else if g.key < 0.5 {
        "ESPN2"
    } else {
        "ESPNU"
    }
}

fn playoff(g: &CollegeGame, round: u8) -> &'static str {
    let y = g.year;
    if fcs_game(g) {
        // The FCS playoffs: early rounds on ESPN+, the semifinals on ESPN2.
        return if round >= 3 {
            "ESPN2"
        } else if y >= 2019 {
            "ESPN+"
        } else {
            "ESPN3"
        };
    }
    if g.home_div != Division::FBS {
        return if y >= 2012 { "School stream" } else { "Local radio" };
    }
    // The 12-team playoff's first round on campus (2024): two on TNT.
    if y >= 2024 && round == 1 {
        return if g.key < 0.5 { "TNT" } else { "ESPN" };
    }
    if let Some(n) = g.name {
        return bowl(&CollegeGame { name: Some(n), draw: 1.0, ..*g });
    }
    "ESPN"
}

fn title(g: &CollegeGame) -> &'static str {
    let y = g.year;
    if fcs_game(g) {
        return match y {
            2023.. => "ABC",
            2010..=2022 => "ESPN2",
            _ => "ESPN",
        };
    }
    match y {
        2014.. => "ESPN",
        2011..=2013 => "ESPN",
        2007..=2010 => "FOX",
        1999..=2006 => "ABC",
        _ => "ABC",
    }
}

fn fcs_game(g: &CollegeGame) -> bool {
    g.home_div == Division::FCS || g.away_div == Division::FCS
}

/// An NFL game as the networks see it.
pub struct NflGame<'a> {
    pub year: i32,
    pub date: Date,
    /// Kickoff, minutes after midnight Eastern.
    pub minute: u16,
    /// The game's window (`nfl_year::Window::name`): "Sunday afternoon",
    /// "Thursday night", "Sunday night", "Monday night", "Saturday",
    /// "Thanksgiving", "Black Friday", "Christmas", "an international game",
    /// "the season opener", "a special date".
    pub window: &'a str,
    /// The road team's conference: true for the AFC.
    pub away_afc: bool,
    pub home_afc: bool,
    /// A playoff round (1 wild card .. 4 the Super Bowl).
    pub round: Option<u32>,
    pub key: f64,
}

/// The network that carries an NFL game.
pub fn nfl(g: &NflGame) -> &'static str {
    let y = g.year;
    // Sunday afternoons: the road team's conference's network.
    let afternoon = |away_afc: bool| -> &'static str {
        match (away_afc, y) {
            (true, 1998..) => "CBS",
            (true, _) => "NBC",
            (false, 1994..) => "FOX",
            (false, _) => "CBS",
        }
    };
    if let Some(r) = g.round {
        return match r {
            4 => super_bowl(y),
            3 => afternoon(g.home_afc),
            2 => {
                if y >= 2022 && g.key < 0.25 {
                    "ESPN/ABC"
                } else if y >= 2014 && g.key > 0.75 {
                    "NBC"
                } else {
                    afternoon(g.home_afc)
                }
            }
            _ => {
                // Six wild card games from 2020, one on each network and a
                // streaming exclusive from 2023.
                let pick = (g.key * 6.0) as usize;
                match (y, pick) {
                    (2024.., 5) => "Prime Video",
                    (2023, 5) => "Peacock",
                    (2015.., 4) => "ESPN/ABC",
                    (2014.., 3) => "NBC",
                    _ => afternoon(g.home_afc),
                }
            }
        };
    }
    let local = g.minute / 60;
    match g.window {
        "Sunday night" => match y {
            2006.. => "NBC · SNF",
            1990..=1997 if g.date.month() <= 10 => "TNT",
            1987..=2005 => "ESPN · SNF",
            _ => afternoon(g.away_afc),
        },
        "Monday night" => match y {
            2006.. => {
                if y >= 2020 && g.key < 0.2 {
                    "ESPN/ABC · MNF"
                } else {
                    "ESPN · MNF"
                }
            }
            _ => "ABC · MNF",
        },
        "Thursday night" => match y {
            2022.. => "Prime Video · TNF",
            2018..=2021 => "FOX · TNF",
            2016..=2017 => {
                if g.date.month() <= 10 {
                    "CBS · TNF"
                } else {
                    "NBC · TNF"
                }
            }
            2014..=2015 => {
                if g.date.month() <= 10 {
                    "CBS · TNF"
                } else {
                    "NFL Network"
                }
            }
            2006..=2013 => "NFL Network",
            _ => "ESPN",
        },
        "the season opener" => {
            if y >= 2006 {
                "NBC"
            } else {
                "ABC"
            }
        }
        "Thanksgiving" => {
            if local >= 20 {
                if y >= 2012 { "NBC" } else { "NFL Network" }
            } else {
                afternoon(g.away_afc)
            }
        }
        "Black Friday" => {
            if y >= 2023 {
                "Prime Video"
            } else {
                afternoon(g.away_afc)
            }
        }
        "Christmas" => {
            if y >= 2024 {
                "Netflix"
            } else if local >= 20 {
                "NBC"
            } else {
                afternoon(g.away_afc)
            }
        }
        "Saturday" => {
            if y >= 2022 && g.date.month() == 1 {
                "ESPN/ABC"
            } else if y >= 2006 {
                "NFL Network"
            } else {
                afternoon(g.away_afc)
            }
        }
        // London's morning games on the league's own channel.
        "an international game" => {
            if local < 11 && y >= 2014 {
                "NFL Network"
            } else {
                afternoon(g.away_afc)
            }
        }
        _ => {
            if g.date.weekday() == Weekday::Sun {
                afternoon(g.away_afc)
            } else if y >= 2024 && g.date.weekday() == Weekday::Fri {
                "Peacock"
            } else {
                "NFL Network"
            }
        }
    }
}

/// The Super Bowl's network by season, as each game really went: CBS and
/// NBC in turn to the 1983 season, ABC joining the turn from 1984, FOX from
/// 1996, CBS, FOX and NBC from 2006, and ESPN and ABC in the turn from the
/// 2026 season.
fn super_bowl(season: i32) -> &'static str {
    match season {
        ..=1983 => {
            if season % 2 == 0 {
                "NBC"
            } else {
                "CBS"
            }
        }
        1984 | 1987 | 1990 | 1994 | 1999 | 2002 | 2005 => "ABC",
        1985 | 1988 | 1992 | 1993 | 1995 | 1997 => "NBC",
        1986 | 1989 | 1991 | 2000 | 2003 => "CBS",
        1996 | 1998 | 2001 | 2004 => "FOX",
        2006..=2013 => ["CBS", "FOX", "NBC"][(season - 2006).rem_euclid(3) as usize],
        2014 | 2017 | 2021 | 2025 => "NBC",
        2015 | 2018 | 2020 | 2023 => "CBS",
        2016 | 2019 | 2022 | 2024 => "FOX",
        _ => ["ESPN/ABC", "CBS", "FOX", "NBC"][(season - 2026).rem_euclid(4) as usize],
    }
}

/// A high school's game on the air: the town's radio station, and from
/// 2014 the NFHS Network's stream for most schools (the cameras spread
/// through 2020). The station's call letters and dial are the school's own,
/// fixed (W east of the Mississippi, K west).
pub fn high_school(year: i32, west: bool, resources: f64, key: u64) -> String {
    let u = |salt: u64| {
        let mut x = key.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ salt.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 31;
        x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 29;
        (x >> 11) as f64 / (1u64 << 53) as f64
    };
    let letters: String = (0..3).map(|i| (b'A' + (u(10 + i) * 26.0) as u8 % 26) as char).collect();
    let call = format!("{}{letters}", if west { 'K' } else { 'W' });
    // FM from the 1970s for most towns; an AM station where the town has one.
    let dial = if u(1) < 0.7 {
        let tenths = 881 + 2 * ((u(2) * 99.0) as u32);
        format!("{}.{} FM", tenths / 10, tenths % 10)
    } else {
        format!("{} AM", 540 + 10 * ((u(3) * 106.0) as u32))
    };
    // About two schools in three have a station that carries them.
    let radio = u(4) < 0.55 + 0.3 * resources.clamp(0.0, 1.0);
    // The stream's spread: a few schools in 2014, most by 2021.
    let share = match year {
        ..=2013 => 0.0,
        2014..=2016 => 0.2,
        2017..=2019 => 0.45,
        2020 => 0.7,
        _ => 0.9,
    } * (0.7 + 0.3 * resources.clamp(0.0, 1.0));
    let stream = u(5) < share;
    match (radio, stream) {
        (true, true) => format!("{call} {dial} · NFHS stream"),
        (true, false) => format!("{call} {dial}"),
        (false, true) => "NFHS Network stream".to_string(),
        (false, false) => "Not carried".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game<'a>(year: i32, conf: &'a str, kick: (u16, u16), draw: f64) -> CollegeGame<'a> {
        let sat = Date::ymd(year, 10, 1).on_or_after(Weekday::Sat);
        CollegeGame {
            year,
            date: sat,
            kick,
            kind: GameKind::Regular,
            name: None,
            home_conf: conf,
            away_conf: conf,
            home_div: Division::FBS,
            away_div: Division::FBS,
            home: "Somewhere",
            away: "Elsewhere",
            draw,
            key: 0.3,
        }
    }

    #[test]
    fn conference_networks_start_in_their_years() {
        // The SEC Network from 2014, the ACC Network from 2019, the Big Ten
        // Network from 2007.
        assert_eq!(college(&game(2026, "SEC", (12, 0), 0.2)), "SEC Network");
        assert_ne!(college(&game(2012, "SEC", (12, 0), 0.2)), "SEC Network");
        assert_eq!(college(&game(2026, "ACC", (15, 30), 0.2)), "ACC Network");
        assert_eq!(college(&game(2015, "ACC", (15, 30), 0.2)), "Raycom");
        assert_eq!(college(&game(2010, "Big Ten", (12, 0), 0.2)), "Big Ten Network");
        assert_ne!(college(&game(2005, "Big Ten", (12, 0), 0.2)), "Big Ten Network");
    }

    #[test]
    fn the_big_games_go_to_the_big_networks_by_era() {
        // CBS's SEC game at 3:30 through 2023, ABC's after.
        assert_eq!(college(&game(2019, "SEC", (15, 30), 0.9)), "CBS");
        assert_eq!(college(&game(2026, "SEC", (15, 30), 0.9)), "ABC");
        // The Big Ten's three windows from 2023.
        assert_eq!(college(&game(2026, "Big Ten", (12, 0), 0.9)), "FOX");
        assert_eq!(college(&game(2026, "Big Ten", (15, 30), 0.9)), "CBS");
        assert_eq!(college(&game(2026, "Big Ten", (19, 30), 0.9)), "NBC");
    }

    #[test]
    fn notre_dame_and_army_navy_keep_their_networks() {
        let mut g = game(2026, "Independent", (15, 30), 0.5);
        g.home = "Notre Dame";
        assert_eq!(college(&g), "NBC");
        g.home = "Army";
        g.away = "Navy";
        g.home_conf = "American";
        assert_eq!(college(&g), "CBS");
    }

    fn pro(year: i32, window: &str, away_afc: bool) -> NflGame<'_> {
        NflGame {
            year,
            date: Date::ymd(year, 10, 4).on_or_after(Weekday::Sun),
            minute: 13 * 60,
            window,
            away_afc,
            home_afc: !away_afc,
            round: None,
            key: 0.5,
        }
    }

    #[test]
    fn nfl_networks_follow_the_contracts() {
        assert_eq!(nfl(&pro(2026, "Sunday afternoon", true)), "CBS");
        assert_eq!(nfl(&pro(2026, "Sunday afternoon", false)), "FOX");
        assert_eq!(nfl(&pro(1992, "Sunday afternoon", false)), "CBS");
        assert_eq!(nfl(&pro(1996, "Sunday afternoon", true)), "NBC");
        assert_eq!(nfl(&pro(2026, "Sunday night", true)), "NBC · SNF");
        assert_eq!(nfl(&pro(2000, "Sunday night", true)), "ESPN · SNF");
        assert_eq!(nfl(&pro(2005, "Monday night", true)), "ABC · MNF");
        assert!(nfl(&pro(2026, "Monday night", true)).starts_with("ESPN"));
        assert_eq!(nfl(&pro(2026, "Thursday night", true)), "Prime Video · TNF");
        assert_eq!(nfl(&pro(2019, "Thursday night", true)), "FOX · TNF");
        assert_eq!(nfl(&pro(2010, "Thursday night", true)), "NFL Network");
    }

    #[test]
    fn super_bowls_rotate() {
        assert_eq!(super_bowl(2025), "NBC");
        assert_eq!(super_bowl(2026), "ESPN/ABC");
        assert_eq!(super_bowl(2027), "CBS");
        assert_eq!(super_bowl(2030), "ESPN/ABC");
    }

    #[test]
    fn high_school_radio_is_fixed_and_streams_come_late() {
        let a = high_school(2026, false, 0.6, 77);
        assert_eq!(a, high_school(2026, false, 0.6, 77));
        assert!(!high_school(2010, true, 1.0, 5).contains("stream"));
        // Most schools stream by the 2020s.
        let n = (0..400).filter(|&k| high_school(2026, false, 0.5, k).contains("NFHS")).count();
        assert!(n > 250, "{n}");
    }
}
