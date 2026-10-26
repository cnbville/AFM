//! The other pro leagues as full leagues (final build X7, later extras
//! topic 5): the CFL, arena football, NFL Europe, the three XFLs, the AAF,
//! the USFL of 2022-23 and both UFLs, each with its real teams by era, a
//! schedule and playoffs on its real dates, a head coach and coordinators
//! in the coaching carousel, rosters from the street (the CFL's from its
//! own draft and national pool too), and every game on the engine under
//! the league's own rule book (`game::books`). Each player's lines go on
//! his page.
//!
//! The leagues run in full from 1996, the earliest a world's history
//! starts; the team lists by year are Claude's reading of each league's
//! history (see `docs/world-model.md`, "Final build lane 4").

use std::collections::{HashMap, HashSet};

use rayon::prelude::*;

use super::Status;
use super::leagues::{LEAGUES, USFL_2022};
use crate::calendar::{Date, Kind, Loudness};
use crate::game::books::Book;
use crate::game::roster::{Caller, OffScheme, Staff as GameStaff};
use crate::game::{GamePlayer, GameResult, GameTeam, Venue};
use crate::positions::Position;
use crate::rng::{Rng, clamp};
use crate::world::coach::{Employer, Role};
use crate::world::season::{key_normal, key_uniform};
use crate::world::{Stage, World};

/// The leagues, by their index in `leagues::LEAGUES` (the USFL of 2022-23
/// comes after them).
pub const CFL: u8 = 0;
pub const AFL1: u8 = 2;
pub const AFL2: u8 = 3;
pub const NFLE: u8 = 5;
pub const XFL01: u8 = 6;
pub const UFL09: u8 = 7;
pub const AAF: u8 = 8;
pub const XFL20: u8 = 9;
pub const XFL23: u8 = 10;
pub const UFL24: u8 = 11;
pub const USFL22: u8 = 12;
pub const LEAGUE_COUNT: usize = 13;

/// The first season a league runs in full (teams, games, coaches): the
/// earliest a world's history starts.
pub const FULL_FROM: i32 = 1996;

/// One team in one stretch of years: its league, its franchise (moves and
/// new names keep the franchise), its name then and its division.
#[derive(Clone, Copy, Debug)]
pub struct TeamDef {
    pub league: u8,
    pub fr: u16,
    pub name: &'static str,
    pub from: i32,
    pub to: i32,
    pub div: u8,
}

const fn t(league: u8, fr: u16, name: &'static str, from: i32, to: i32, div: u8) -> TeamDef {
    TeamDef { league, fr, name, from, to, div }
}

const NOW: i32 = 9999;

/// Every team, by franchise. Divisions: CFL 0 West, 1 East; XFL of 2001
/// and 2020 0 East, 1 West; XFL of 2023 0 North, 1 South; USFL 0 North,
/// 1 South; UFL 0 USFL Conference, 1 XFL Conference; AAF 0 East, 1 West.
pub const TEAMS: &[TeamDef] = &[
    // The CFL (Winnipeg played in the East 1987-2001 and 2006-13).
    t(CFL, 0, "BC Lions", 1958, NOW, 0),
    t(CFL, 1, "Calgary Stampeders", 1958, NOW, 0),
    t(CFL, 2, "Edmonton Eskimos", 1958, 2020, 0),
    t(CFL, 2, "Edmonton Elks", 2021, NOW, 0),
    t(CFL, 3, "Saskatchewan Roughriders", 1958, NOW, 0),
    t(CFL, 4, "Winnipeg Blue Bombers", 1958, 1986, 0),
    t(CFL, 4, "Winnipeg Blue Bombers", 1987, 2001, 1),
    t(CFL, 4, "Winnipeg Blue Bombers", 2002, 2005, 0),
    t(CFL, 4, "Winnipeg Blue Bombers", 2006, 2013, 1),
    t(CFL, 4, "Winnipeg Blue Bombers", 2014, NOW, 0),
    t(CFL, 5, "Hamilton Tiger-Cats", 1958, NOW, 1),
    t(CFL, 6, "Toronto Argonauts", 1958, NOW, 1),
    t(CFL, 7, "Ottawa Rough Riders", 1958, 1996, 1),
    t(CFL, 8, "Ottawa Renegades", 2002, 2005, 1),
    t(CFL, 9, "Ottawa Redblacks", 2014, NOW, 1),
    t(CFL, 10, "Montreal Alouettes", 1996, NOW, 1),
    // The Arena Football League, 1987-2008.
    t(AFL1, 11, "Albany Firebirds", 1990, 1999, 0),
    t(AFL1, 11, "Indiana Firebirds", 2000, 2004, 0),
    t(AFL1, 12, "Arizona Rattlers", 1992, 2008, 0),
    t(AFL1, 13, "Orlando Predators", 1991, 2008, 0),
    t(AFL1, 14, "Tampa Bay Storm", 1991, 2008, 0),
    t(AFL1, 15, "Iowa Barnstormers", 1995, 2000, 0),
    t(AFL1, 15, "New York Dragons", 2001, 2008, 0),
    t(AFL1, 16, "Milwaukee Mustangs", 1994, 2001, 0),
    t(AFL1, 17, "San Jose SaberCats", 1995, 2008, 0),
    t(AFL1, 18, "Florida Bobcats", 1996, 2001, 0),
    t(AFL1, 19, "Anaheim Piranhas", 1996, 1997, 0),
    t(AFL1, 20, "Charlotte Rage", 1992, 1996, 0),
    t(AFL1, 21, "Connecticut Coyotes", 1995, 1996, 0),
    t(AFL1, 22, "Memphis Pharaohs", 1995, 1996, 0),
    t(AFL1, 22, "Portland Forest Dragons", 1997, 1999, 0),
    t(AFL1, 22, "Oklahoma Wranglers", 2000, 2001, 0),
    t(AFL1, 23, "Minnesota Fighting Pike", 1996, 1996, 0),
    t(AFL1, 24, "St. Louis Stampede", 1995, 1996, 0),
    t(AFL1, 25, "Texas Terror", 1996, 1997, 0),
    t(AFL1, 25, "Houston ThunderBears", 1998, 2001, 0),
    t(AFL1, 26, "Nashville Kats", 1997, 2001, 0),
    t(AFL1, 26, "Georgia Force", 2002, 2008, 0),
    t(AFL1, 27, "New Jersey Red Dogs", 1997, 2000, 0),
    t(AFL1, 27, "New Jersey Gladiators", 2001, 2002, 0),
    t(AFL1, 27, "Las Vegas Gladiators", 2003, 2007, 0),
    t(AFL1, 27, "Cleveland Gladiators", 2008, 2008, 0),
    t(AFL1, 28, "New York CityHawks", 1997, 1998, 0),
    t(AFL1, 28, "New England Sea Wolves", 1999, 2000, 0),
    t(AFL1, 28, "Toronto Phantoms", 2001, 2002, 0),
    t(AFL1, 29, "Grand Rapids Rampage", 1998, 2008, 0),
    t(AFL1, 30, "Buffalo Destroyers", 1999, 2003, 0),
    t(AFL1, 30, "Columbus Destroyers", 2004, 2008, 0),
    t(AFL1, 31, "Carolina Cobras", 2000, 2004, 0),
    t(AFL1, 32, "Los Angeles Avengers", 2000, 2008, 0),
    t(AFL1, 33, "Chicago Rush", 2001, 2008, 0),
    t(AFL1, 34, "Detroit Fury", 2001, 2004, 0),
    t(AFL1, 35, "Dallas Desperados", 2002, 2008, 0),
    t(AFL1, 36, "Colorado Crush", 2003, 2008, 0),
    t(AFL1, 37, "Philadelphia Soul", 2004, 2008, 0),
    t(AFL1, 38, "Austin Wranglers", 2004, 2007, 0),
    t(AFL1, 39, "New Orleans VooDoo", 2004, 2005, 0),
    t(AFL1, 39, "New Orleans VooDoo", 2007, 2008, 0),
    t(AFL1, 40, "Nashville Kats", 2005, 2007, 0),
    t(AFL1, 41, "Kansas City Brigade", 2006, 2008, 0),
    t(AFL1, 42, "Utah Blaze", 2006, 2008, 0),
    // The Arena Football League, 2010-2019.
    t(AFL2, 43, "Alabama Vipers", 2010, 2010, 0),
    t(AFL2, 44, "Arizona Rattlers", 2010, 2016, 0),
    t(AFL2, 45, "Bossier-Shreveport Battle Wings", 2010, 2010, 0),
    t(AFL2, 46, "Chicago Rush", 2010, 2013, 0),
    t(AFL2, 47, "Cleveland Gladiators", 2010, 2017, 0),
    t(AFL2, 48, "Dallas Vigilantes", 2010, 2011, 0),
    t(AFL2, 49, "Iowa Barnstormers", 2010, 2014, 0),
    t(AFL2, 50, "Jacksonville Sharks", 2010, 2016, 0),
    t(AFL2, 51, "Milwaukee Iron", 2010, 2010, 0),
    t(AFL2, 51, "Milwaukee Mustangs", 2011, 2012, 0),
    t(AFL2, 52, "Oklahoma City Yard Dawgz", 2010, 2010, 0),
    t(AFL2, 53, "Orlando Predators", 2010, 2016, 0),
    t(AFL2, 54, "Spokane Shock", 2010, 2015, 0),
    t(AFL2, 55, "Tampa Bay Storm", 2010, 2017, 0),
    t(AFL2, 56, "Tulsa Talons", 2010, 2011, 0),
    t(AFL2, 56, "San Antonio Talons", 2012, 2014, 0),
    t(AFL2, 57, "Utah Blaze", 2010, 2013, 0),
    t(AFL2, 58, "Georgia Force", 2011, 2012, 0),
    t(AFL2, 59, "Kansas City Command", 2011, 2012, 0),
    t(AFL2, 60, "New Orleans VooDoo", 2011, 2015, 0),
    t(AFL2, 61, "Philadelphia Soul", 2011, 2019, 0),
    t(AFL2, 62, "Pittsburgh Power", 2011, 2014, 0),
    t(AFL2, 63, "San Jose SaberCats", 2011, 2015, 0),
    t(AFL2, 64, "Los Angeles KISS", 2014, 2016, 0),
    t(AFL2, 65, "Portland Thunder", 2014, 2015, 0),
    t(AFL2, 65, "Portland Steel", 2016, 2016, 0),
    t(AFL2, 66, "Las Vegas Outlaws", 2015, 2015, 0),
    t(AFL2, 67, "Baltimore Brigade", 2017, 2019, 0),
    t(AFL2, 68, "Washington Valor", 2017, 2019, 0),
    t(AFL2, 69, "Albany Empire", 2018, 2019, 0),
    t(AFL2, 70, "Atlantic City Blackjacks", 2019, 2019, 0),
    t(AFL2, 71, "Columbus Destroyers", 2019, 2019, 0),
    // NFL Europe (the World League to 1997, NFL Europa in 2007).
    t(NFLE, 72, "Amsterdam Admirals", 1995, 2007, 0),
    t(NFLE, 73, "Barcelona Dragons", 1995, 2003, 0),
    t(NFLE, 74, "Frankfurt Galaxy", 1995, 2007, 0),
    t(NFLE, 75, "London Monarchs", 1995, 1997, 0),
    t(NFLE, 75, "England Monarchs", 1998, 1998, 0),
    t(NFLE, 76, "Rhein Fire", 1995, 2007, 0),
    t(NFLE, 77, "Scottish Claymores", 1995, 2004, 0),
    t(NFLE, 78, "Berlin Thunder", 1999, 2007, 0),
    t(NFLE, 79, "Cologne Centurions", 2004, 2007, 0),
    t(NFLE, 80, "Hamburg Sea Devils", 2005, 2007, 0),
    // The XFL of 2001.
    t(XFL01, 81, "NY/NJ Hitmen", 2001, 2001, 0),
    t(XFL01, 82, "Orlando Rage", 2001, 2001, 0),
    t(XFL01, 83, "Birmingham Thunderbolts", 2001, 2001, 0),
    t(XFL01, 84, "Chicago Enforcers", 2001, 2001, 0),
    t(XFL01, 85, "Los Angeles Xtreme", 2001, 2001, 1),
    t(XFL01, 86, "Las Vegas Outlaws", 2001, 2001, 1),
    t(XFL01, 87, "San Francisco Demons", 2001, 2001, 1),
    t(XFL01, 88, "Memphis Maniax", 2001, 2001, 1),
    // The UFL of 2009-2012.
    t(UFL09, 89, "California Redwoods", 2009, 2009, 0),
    t(UFL09, 89, "Sacramento Mountain Lions", 2010, 2012, 0),
    t(UFL09, 90, "Florida Tuskers", 2009, 2010, 0),
    t(UFL09, 90, "Virginia Destroyers", 2011, 2012, 0),
    t(UFL09, 91, "Las Vegas Locomotives", 2009, 2012, 0),
    t(UFL09, 92, "New York Sentinels", 2009, 2009, 0),
    t(UFL09, 92, "Hartford Colonials", 2010, 2010, 0),
    t(UFL09, 93, "Omaha Nighthawks", 2010, 2012, 0),
    // The Alliance of American Football (2019).
    t(AAF, 94, "Atlanta Legends", 2019, 2019, 0),
    t(AAF, 95, "Birmingham Iron", 2019, 2019, 0),
    t(AAF, 96, "Memphis Express", 2019, 2019, 0),
    t(AAF, 97, "Orlando Apollos", 2019, 2019, 0),
    t(AAF, 98, "Arizona Hotshots", 2019, 2019, 1),
    t(AAF, 99, "Salt Lake Stallions", 2019, 2019, 1),
    t(AAF, 100, "San Antonio Commanders", 2019, 2019, 1),
    t(AAF, 101, "San Diego Fleet", 2019, 2019, 1),
    // The XFL of 2020.
    t(XFL20, 102, "DC Defenders", 2020, 2020, 0),
    t(XFL20, 103, "New York Guardians", 2020, 2020, 0),
    t(XFL20, 104, "St. Louis BattleHawks", 2020, 2020, 0),
    t(XFL20, 105, "Tampa Bay Vipers", 2020, 2020, 0),
    t(XFL20, 106, "Dallas Renegades", 2020, 2020, 1),
    t(XFL20, 107, "Houston Roughnecks", 2020, 2020, 1),
    t(XFL20, 108, "Los Angeles Wildcats", 2020, 2020, 1),
    t(XFL20, 109, "Seattle Dragons", 2020, 2020, 1),
    // The XFL of 2023.
    t(XFL23, 110, "DC Defenders", 2023, 2023, 0),
    t(XFL23, 111, "Seattle Sea Dragons", 2023, 2023, 0),
    t(XFL23, 112, "St. Louis Battlehawks", 2023, 2023, 0),
    t(XFL23, 113, "Vegas Vipers", 2023, 2023, 0),
    t(XFL23, 114, "Arlington Renegades", 2023, 2023, 1),
    t(XFL23, 115, "Houston Roughnecks", 2023, 2023, 1),
    t(XFL23, 116, "Orlando Guardians", 2023, 2023, 1),
    t(XFL23, 117, "San Antonio Brahmas", 2023, 2023, 1),
    // The UFL from 2024.
    t(UFL24, 118, "Birmingham Stallions", 2024, NOW, 0),
    t(UFL24, 119, "Houston Roughnecks", 2024, NOW, 0),
    t(UFL24, 120, "Memphis Showboats", 2024, NOW, 0),
    t(UFL24, 121, "Michigan Panthers", 2024, NOW, 0),
    t(UFL24, 122, "Arlington Renegades", 2024, NOW, 1),
    t(UFL24, 123, "DC Defenders", 2024, NOW, 1),
    t(UFL24, 124, "San Antonio Brahmas", 2024, NOW, 1),
    t(UFL24, 125, "St. Louis Battlehawks", 2024, NOW, 1),
    // The USFL of 2022-23.
    t(USFL22, 126, "Michigan Panthers", 2022, 2023, 0),
    t(USFL22, 127, "New Jersey Generals", 2022, 2023, 0),
    t(USFL22, 128, "Philadelphia Stars", 2022, 2023, 0),
    t(USFL22, 129, "Pittsburgh Maulers", 2022, 2023, 0),
    t(USFL22, 130, "Birmingham Stallions", 2022, 2023, 1),
    t(USFL22, 131, "Houston Gamblers", 2022, 2023, 1),
    t(USFL22, 132, "New Orleans Breakers", 2022, 2023, 1),
    t(USFL22, 133, "Tampa Bay Bandits", 2022, 2022, 1),
    t(USFL22, 133, "Memphis Showboats", 2023, 2023, 1),
];

/// Franchises in `TEAMS`.
pub const FRANCHISES: usize = 134;

/// The league's name for a season (the World League's name to 1997, NFL
/// Europa in 2007).
pub fn league_name(k: u8, year: i32) -> &'static str {
    match k {
        NFLE if year <= 1997 => "World League",
        NFLE if year == 2007 => "NFL Europa",
        USFL22 => USFL_2022.name,
        _ => LEAGUES.get(k as usize).map_or("", |l| l.name),
    }
}

/// The short name for tables and lines.
pub fn league_short(k: u8) -> &'static str {
    match k {
        CFL => "CFL",
        AFL1 | AFL2 => "AFL",
        NFLE => "NFLE",
        XFL01 | XFL20 | XFL23 => "XFL",
        UFL09 | UFL24 => "UFL",
        AAF => "AAF",
        USFL22 => "USFL",
        _ => "",
    }
}

/// The rule book a league plays under.
pub fn book(k: u8) -> Book {
    match k {
        CFL => Book::Cfl,
        AFL1 | AFL2 => Book::Arena,
        NFLE => Book::NflEurope,
        XFL01 => Book::Xfl2001,
        XFL20 | XFL23 => Book::Xfl,
        UFL09 => Book::Ufl2009,
        AAF => Book::Aaf,
        UFL24 => Book::Ufl,
        USFL22 => Book::Usfl2022,
        _ => Book::Usfl1983,
    }
}

/// The franchise's team in a season.
pub fn def(fr: usize, year: i32) -> Option<&'static TeamDef> {
    TEAMS.iter().find(|d| d.fr as usize == fr && d.from <= year && year <= d.to)
}

/// The franchise's name in a season, or its last name.
pub fn name(fr: usize, year: i32) -> &'static str {
    def(fr, year)
        .or_else(|| TEAMS.iter().filter(|d| d.fr as usize == fr).min_by_key(|d| (d.from - year).abs()))
        .map_or("", |d| d.name)
}

/// Nicknames of two words ("Blue Bombers", "Sea Devils").
const TWO_WORDS: &[&str] = &[
    "Battle Wings",
    "Blue Bombers",
    "Fighting Pike",
    "Forest Dragons",
    "Mountain Lions",
    "Red Dogs",
    "Rough Riders",
    "Sea Devils",
    "Sea Dragons",
    "Sea Wolves",
    "Yard Dawgz",
];

/// The franchise's nickname in a season ("Elks", "Blue Bombers").
pub fn nickname(fr: usize, year: i32) -> &'static str {
    let n = name(fr, year);
    match TWO_WORDS.iter().find(|t| n.ends_with(*t)) {
        Some(t) => &n[n.len() - t.len()..],
        None => n.rsplit_once(' ').map_or(n, |x| x.1),
    }
}

/// The franchise's city in a season (its name less the nickname).
pub fn city(fr: usize, year: i32) -> &'static str {
    let n = name(fr, year);
    n.strip_suffix(nickname(fr, year)).map_or(n, str::trim_end)
}

/// The franchise's league.
pub fn league_of(fr: usize) -> u8 {
    TEAMS.iter().find(|d| d.fr as usize == fr).map_or(0, |d| d.league)
}

/// The franchises playing in a league's season.
pub fn teams_in(k: u8, year: i32) -> Vec<usize> {
    let mut v: Vec<usize> =
        TEAMS.iter().filter(|d| d.league == k && d.from <= year && year <= d.to).map(|d| d.fr as usize).collect();
    v.dedup();
    v
}

/// Whether a league plays its season in full in the world this year.
pub fn full(k: u8, year: i32) -> bool {
    year >= FULL_FROM && format(k, year).is_some() && !teams_in(k, year).is_empty()
}

/// How a league's playoffs run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Playoff {
    /// The season stopped before its end: no champion.
    None,
    /// The CFL: three from each division (a fourth crosses over when it
    /// has the better record), the semi-finals, the finals, the Grey Cup.
    Cfl,
    /// The best `n` by record, seeded, one game each round.
    Top(u8),
    /// The best two of each division meet, then the winners.
    PerDiv,
}

/// A league's season: games a team, the first game, the week the season
/// stopped (if it did), the playoffs, the roster size and the title game.
#[derive(Clone, Copy, Debug)]
pub struct Format {
    pub games: u8,
    pub start: Date,
    pub stop_after: Option<u8>,
    pub playoff: Playoff,
    pub roster: usize,
    pub title: &'static str,
}

/// A league's season by year (real lengths and dates; no CFL season in
/// 2020, a 14-game one from August in 2021; the XFL of 2020 stopped after
/// five weeks, the AAF after eight, the UFL of 2012 after four).
pub fn format(k: u8, year: i32) -> Option<Format> {
    let d = |m: u32, day: u32| Date::ymd(year, m, day);
    let f = |games: u8, start: Date, stop: Option<u8>, playoff: Playoff, roster: usize, title: &'static str| Format {
        games,
        start,
        stop_after: stop,
        playoff,
        roster,
        title,
    };
    Some(match k {
        CFL if year == 2020 => return None,
        CFL if year == 2021 => f(14, d(8, 5), None, Playoff::Cfl, 46, "Grey Cup"),
        CFL => f(18, d(6, 12), None, Playoff::Cfl, 46, "Grey Cup"),
        AFL1 => {
            let games = if year <= 2000 { 14 } else { 16 };
            let start = if year <= 2000 { d(4, 25) } else { d(2, 27) };
            f(games, start, None, Playoff::Top(8), 24, "ArenaBowl")
        }
        AFL2 => {
            let games = match year {
                2010 | 2016 => 16,
                2011..=2015 => 18,
                2017 => 14,
                _ => 12,
            };
            let n = teams_in(k, year).len();
            f(games, d(3, 26), None, Playoff::Top(if n >= 8 { 8 } else { 4 }), 24, "ArenaBowl")
        }
        NFLE => f(10, d(4, 5), None, Playoff::Top(2), 46, "World Bowl"),
        XFL01 => f(10, d(2, 3), None, Playoff::PerDiv, 45, "Million Dollar Game"),
        UFL09 => match year {
            2009 => f(6, d(10, 8), None, Playoff::Top(2), 46, "UFL Championship"),
            2010 => f(8, d(9, 18), None, Playoff::Top(2), 46, "UFL Championship"),
            2011 => f(4, d(9, 17), None, Playoff::Top(2), 46, "UFL Championship"),
            _ => f(8, d(9, 26), Some(4), Playoff::None, 46, "UFL Championship"),
        },
        AAF => f(10, d(2, 9), Some(8), Playoff::None, 46, "AAF Championship"),
        XFL20 => f(10, d(2, 8), Some(5), Playoff::None, 46, "XFL Championship"),
        XFL23 => f(10, d(2, 18), None, Playoff::PerDiv, 46, "XFL Championship"),
        UFL24 => f(10, d(3, 29), None, Playoff::PerDiv, 46, "UFL Championship"),
        USFL22 => f(10, d(4, 16), None, Playoff::PerDiv, 46, "USFL Championship"),
        _ => return None,
    })
}

/// A team's line in the standings.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Row {
    pub fr: u16,
    pub w: u8,
    pub l: u8,
    pub t: u8,
    pub pf: u16,
    pub pa: u16,
}

impl Row {
    pub fn pct(&self) -> f64 {
        let g = (self.w + self.l + self.t) as f64;
        if g == 0.0 { 0.0 } else { (self.w as f64 + 0.5 * self.t as f64) / g }
    }
    /// The CFL ranks by points: two a win, one a tie.
    pub fn points(&self) -> u32 {
        2 * self.w as u32 + self.t as u32
    }
}

/// One game of a season.
#[derive(Clone, Copy, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Game {
    pub week: u8,
    pub home: u16,
    pub away: u16,
    pub score: [u16; 2],
    /// 0 regular season, else the playoff round (the last is the title).
    pub round: u8,
}

/// A league season under way.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Live {
    pub league: u8,
    pub year: i32,
    pub weeks: Vec<Vec<(u16, u16)>>,
    /// The next week to play (regular season, then playoff rounds).
    pub week: u16,
    pub table: Vec<Row>,
    pub games: Vec<Game>,
    /// The teams still alive in the playoffs, best seed first, once they
    /// start; the round being played.
    pub alive: Vec<u16>,
    pub round: u8,
    pub done: bool,
}

/// A finished league season, for the record books and the screens.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Season {
    pub league: u8,
    pub year: i32,
    pub table: Vec<Row>,
    pub champion: Option<u16>,
    pub runner_up: Option<u16>,
    pub final_score: [u16; 2],
    /// Why there was no champion (a season stopped short).
    #[serde(deserialize_with = "crate::save::leaked")]
    pub note: crate::save::Text,
    /// The season's games (kept for the last two seasons only).
    pub games: Vec<Game>,
}

/// A franchise in the world: its roster (people), its staff and titles.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Team {
    pub roster: Vec<u32>,
    pub head_coach: Option<usize>,
    pub oc: Option<usize>,
    pub dc: Option<usize>,
    pub titles: u16,
    /// Each finished season's share of wins, most recent last (the owner's
    /// read of his coach).
    pub win_share: Vec<f32>,
}

/// The other leagues in the world.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Others {
    pub teams: Vec<Team>,
    pub live: Vec<Live>,
    pub seasons: Vec<Season>,
    /// Canadian ("national") players by player id, with the school each
    /// came from (`U_SPORTS`): the CFL's ratio counts them.
    pub nationals: HashMap<u32, u8>,
    /// The CFL draft, every pick of every year.
    pub draft: Vec<Pick>,
}

/// A pick in the CFL draft (the name kept: the record outlives him).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Pick {
    pub year: i32,
    pub pick: u16,
    pub team: u16,
    pub id: u32,
    pub name: String,
    pub pos: Position,
    pub school: u8,
}

impl Others {
    pub fn team(&self, fr: usize) -> Option<&Team> {
        self.teams.get(fr)
    }
    fn ensure(&mut self) {
        if self.teams.len() < FRANCHISES {
            self.teams.resize(FRANCHISES, Team::default());
        }
    }
    /// The franchise a player is on now, if any (by his place in the
    /// world's people).
    pub fn team_of(&self, i: usize) -> Option<usize> {
        self.teams.iter().position(|t| t.roster.contains(&(i as u32)))
    }
    /// The people moved up at the yearly compaction: rosters follow them,
    /// and anyone who left football leaves his roster.
    pub(crate) fn remap(&mut self, map: &[Option<usize>]) {
        for t in self.teams.iter_mut() {
            t.roster =
                t.roster.iter().filter_map(|&i| map.get(i as usize).copied().flatten().map(|j| j as u32)).collect();
        }
    }
    /// Whether a player is a Canadian (a national under the CFL's ratio).
    pub fn national(&self, id: u32) -> bool {
        self.nationals.contains_key(&id)
    }
}

/// A season of a player in another league: where, how much, his lines.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OtherSeason {
    pub year: i32,
    pub league: u8,
    pub team: u16,
    pub games: u16,
    pub starts: u16,
    pub cmp: u16,
    pub att: u16,
    pub pass_yds: f32,
    pub pass_td: u16,
    pub int: u16,
    pub rush: u16,
    pub rush_yds: f32,
    pub rush_td: u16,
    pub rec: u16,
    pub rec_yds: f32,
    pub rec_td: u16,
    pub tackles: u16,
    pub sacks: f32,
    pub ints: u16,
    pub fg: u16,
    pub fg_att: u16,
    /// What the season paid (US dollars of the day).
    pub pay: f32,
}

impl OtherSeason {
    /// The season's line in words, by position.
    pub fn line(&self, pos: Position) -> String {
        use Position as P;
        if self.games == 0 {
            return "Did not play".into();
        }
        match pos {
            P::QB => format!(
                "{}/{} passing, {:.0} yards, {} TD, {} INT",
                self.cmp, self.att, self.pass_yds, self.pass_td, self.int
            ),
            P::RB => format!("{} carries, {:.0} yards, {} TD", self.rush, self.rush_yds, self.rush_td),
            P::WR | P::TE => format!("{} catches, {:.0} yards, {} TD", self.rec, self.rec_yds, self.rec_td),
            P::OT | P::IOL => format!("{} starts on the line", self.starts),
            P::EDGE | P::DT | P::LB => format!("{} tackles, {:.1} sacks", self.tackles, self.sacks),
            P::CB | P::S => format!("{} tackles, {} INT", self.tackles, self.ints),
            P::K => format!("{}/{} field goals", self.fg, self.fg_att),
            P::P | P::LS => format!("{} games on special teams", self.games),
        }
    }
}

/// U Sports programs the CFL drafts from (with a share of Canadians at
/// American colleges, named as such).
pub const U_SPORTS: [&str; 24] = [
    "Laval",
    "Montréal",
    "Western",
    "Calgary",
    "UBC",
    "Saskatchewan",
    "Alberta",
    "Manitoba",
    "Regina",
    "Guelph",
    "McMaster",
    "Ottawa",
    "Carleton",
    "Queen's",
    "Wilfrid Laurier",
    "Waterloo",
    "Windsor",
    "York",
    "Bishop's",
    "Concordia",
    "McGill",
    "Sherbrooke",
    "Saint Mary's",
    "an American college",
];

/// Roster places by position: the CFL's 46 (a fifth receiver, few tight
/// ends), arena football's 24 (three linemen a side, a fullback), the
/// spring leagues' 46 on NFL lines.
fn places(k: u8, roster: usize) -> Vec<(Position, usize)> {
    use Position::*;
    match k {
        CFL => vec![
            (QB, 3),
            (RB, 3),
            (WR, 8),
            (TE, 1),
            (OT, 4),
            (IOL, 5),
            (EDGE, 4),
            (DT, 4),
            (LB, 4),
            (CB, 5),
            (S, 3),
            (K, 1),
            (P, 1),
        ],
        AFL1 | AFL2 => {
            vec![(QB, 2), (RB, 2), (WR, 5), (OT, 1), (IOL, 4), (EDGE, 2), (DT, 2), (LB, 2), (CB, 3), (S, 1), (K, 1)]
        }
        _ => {
            let _ = roster;
            vec![
                (QB, 3),
                (RB, 3),
                (WR, 6),
                (TE, 3),
                (OT, 4),
                (IOL, 5),
                (EDGE, 4),
                (DT, 4),
                (LB, 4),
                (CB, 5),
                (S, 3),
                (K, 1),
                (P, 1),
            ]
        }
    }
}

/// The level of a league's players (the grade of a typical starter) and
/// the street pool it signs from (proposed, not confirmed: the CFL's and
/// the spring leagues' starters a little below the NFL's last roster
/// spots, arena football's well below).
fn level(k: u8) -> (f64, f64, f64) {
    match k {
        CFL => (57.0, 50.0, 70.0),
        AFL1 | AFL2 => (50.0, 42.0, 60.0),
        _ => (56.0, 50.0, 68.0),
    }
}

/// Pay a season (proposed, not confirmed; real averages): the CFL about
/// $90,000 (Canadian) in the 2020s, half that in the 1990s; arena football
/// about $1,000 a game in the 2010s, more before 2009; the spring leagues
/// about $55,000 (XFL, USFL, UFL) and NFL Europe about $30,000 a spring.
fn pay(k: u8, year: i32, star: bool) -> f64 {
    let era = clamp(0.55 + 0.016 * (year - 1996) as f64, 0.55, 1.15);
    let base = match k {
        CFL => 70_000.0 * era,
        AFL1 => 45_000.0 * era,
        AFL2 => 16_000.0,
        NFLE => 30_000.0 * era,
        _ => 55_000.0,
    };
    if star { base * 2.5 } else { base }
}

/// The other leagues' day: each league's camp, weekly games, playoffs and
/// season's end; the CFL draft in May; the carousel after a season.
pub(crate) fn day(w: &mut World, d: Date) {
    let year = d.year();
    w.others.ensure();
    for k in 0..LEAGUE_COUNT as u8 {
        if !full(k, year) {
            continue;
        }
        let f = format(k, year).unwrap();
        if k == CFL && d == Date::ymd(year, 5, 1) {
            draft(w, d);
        }
        let camp = f.start.add(-21);
        if d == camp {
            open(w, d, k, &f);
        }
        let Some(li) = w.others.live.iter().position(|l| l.league == k && l.year == year && !l.done) else {
            continue;
        };
        let wk = w.others.live[li].week as i32;
        if d == f.start.add(7 * wk) {
            play_week(w, d, li, &f);
        }
    }
}

/// A season opens: the carousel fills empty seats, each team brings back
/// its own players, then signs from the street, and the season is drawn.
fn open(w: &mut World, d: Date, k: u8, f: &Format) {
    let year = d.year();
    let teams = teams_in(k, year);
    carousel(w, k, &teams);
    // Franchises that didn't come back release their players.
    for fr in 0..FRANCHISES {
        if league_of(fr) == k && !teams.contains(&fr) && !w.others.teams[fr].roster.is_empty() {
            let gone = std::mem::take(&mut w.others.teams[fr].roster);
            for i in gone {
                release(w, i as usize, k);
            }
        }
    }
    for &fr in &teams {
        fill(w, d, k, fr, f.roster);
    }
    let weeks = schedule(&teams, f.games, year as u64 ^ (k as u64) << 20 ^ w.options.seed as u64);
    let table = teams.iter().map(|&fr| Row { fr: fr as u16, ..Row::default() }).collect();
    w.others.live.push(Live { league: k, year, weeks, table, ..Live::default() });
    if w.recording {
        let n = teams.len();
        let text = format!(
            "The {} opens camp: {n} teams, {} games each, the first on {}.",
            league_name(k, year),
            f.games,
            f.start.pretty()
        );
        super::market::post(w, d, None, None, Loudness::Small, Kind::News, text);
    }
}

/// The weeks of a season: a round robin (the circle method), repeated,
/// until each team has its games; with an odd number one team rests.
pub(crate) fn schedule(teams: &[usize], games: u8, key: u64) -> Vec<Vec<(u16, u16)>> {
    let mut list: Vec<Option<u16>> = teams.iter().map(|&t| Some(t as u16)).collect();
    if list.len() % 2 == 1 {
        list.push(None);
    }
    let n = list.len();
    // A shuffle from the key, so each season's order differs.
    let mut rng = Rng::new((key ^ key >> 32) as u32);
    for i in (1..n).rev() {
        let j = rng.int(0, i as i32) as usize;
        list.swap(i, j);
    }
    let mut played: HashMap<u16, u8> = HashMap::new();
    let mut weeks = Vec::new();
    let mut r = 0;
    while weeks.len() < 40 && teams.iter().any(|&t| played.get(&(t as u16)).copied().unwrap_or(0) < games) {
        let mut week = Vec::new();
        for i in 0..n / 2 {
            let (a, b) = (list[i], list[n - 1 - i]);
            let (Some(a), Some(b)) = (a, b) else { continue };
            if played.get(&a).copied().unwrap_or(0) >= games || played.get(&b).copied().unwrap_or(0) >= games {
                continue;
            }
            *played.entry(a).or_default() += 1;
            *played.entry(b).or_default() += 1;
            week.push(if (r + i) % 2 == 0 { (a, b) } else { (b, a) });
        }
        if !week.is_empty() {
            weeks.push(week);
        }
        // Rotate all but the first.
        let last = list.pop().unwrap();
        list.insert(1, last);
        r += 1;
    }
    weeks
}

/// A player leaves a team: back to the street.
fn release(w: &mut World, i: usize, k: u8) {
    let p = &mut w.people[i];
    if p.stage != Stage::Pro {
        return;
    }
    if let Some(c) = p.pro.as_mut()
        && c.status == Status::Other(k)
    {
        c.status = Status::FreeAgent;
        c.weeks_unsigned = 0;
        c.sharpness = 1.0;
    }
}

/// Who a team wants for its places: its own players who are still here,
/// then the street by need (best first), then players from below the
/// world's colleges and abroad (generated) where the street runs dry.
fn fill(w: &mut World, d: Date, k: u8, fr: usize, size: usize) {
    let year = d.year();
    let (lvl, lo, hi) = level(k);
    // His own: those still on the roster and still playing (the CFL's carry
    // over under contract; a spring team's come back when unsigned).
    let mut roster: Vec<u32> = w.others.teams[fr]
        .roster
        .iter()
        .copied()
        .filter(|&i| {
            let p = &w.people[i as usize];
            p.stage == Stage::Pro
                && p.player.age <= 36
                && p.pro.as_ref().is_some_and(|c| c.status == Status::FreeAgent || c.status == Status::Other(k))
        })
        .collect();
    let mut have: HashMap<Position, usize> = HashMap::new();
    for &i in &roster {
        *have.entry(w.people[i as usize].player.position).or_default() += 1;
    }
    let need = places(k, size);
    // The street, by position, best first (the consensus read).
    let taken: HashSet<u32> = w.others.teams.iter().flat_map(|t| t.roster.iter().copied()).collect();
    let mut pool: HashMap<Position, Vec<(usize, f64)>> = HashMap::new();
    for i in 0..w.people.len() {
        let p = &w.people[i];
        if p.stage != Stage::Pro || p.player.age > 33 || taken.contains(&(i as u32)) {
            continue;
        }
        let Some(c) = p.pro.as_ref() else { continue };
        if c.status != Status::FreeAgent || c.on_waivers || c.out_weeks > 0 {
            continue;
        }
        let v = super::market::consensus(w, i);
        if !(lo..hi).contains(&v) {
            continue;
        }
        // Not everyone wants to play there (keyed, so the order of the
        // leagues doesn't decide it): about two in three do.
        if key_uniform(w.options.seed as u64 ^ 0x07E7 ^ (year as u64) << 8 ^ k as u64, i as u64) > 0.65 {
            continue;
        }
        pool.entry(p.player.position).or_default().push((i, v));
    }
    for v in pool.values_mut() {
        v.sort_by(|a, b| b.1.total_cmp(&a.1));
    }
    // The CFL's ratio: the Canadians it needs come first, at the spots
    // Canadians fill on real rosters (the interior line, safety,
    // linebacker, the kickers and snappers; few at quarterback or
    // corner), the rest from the street.
    let mut nat_at: HashMap<Position, usize> = HashMap::new();
    if k == CFL {
        let (want, _) = ratio(year);
        for &i in &roster {
            let p = &w.people[i as usize];
            if w.others.national(p.player.id) {
                *nat_at.entry(p.player.position).or_default() += 1;
            }
        }
        let share = |pos: Position| match pos {
            Position::IOL | Position::LS => 0.8,
            Position::K | Position::P | Position::S => 0.6,
            Position::LB | Position::TE => 0.5,
            Position::OT | Position::WR | Position::DT | Position::RB => 0.35,
            Position::EDGE => 0.2,
            Position::CB | Position::QB => 0.15,
        };
        let raw: f64 = need.iter().map(|&(p, n)| share(p) * n as f64).sum();
        let scale = want as f64 / raw.max(1.0);
        for &(pos, n) in &need {
            let target = ((share(pos) * n as f64 * scale).round() as usize).min(n);
            let mut got = have.get(&pos).copied().unwrap_or(0);
            let mut nat = nat_at.get(&pos).copied().unwrap_or(0);
            while nat < target && got < n {
                let key = (fr * 100 + got) as u64 ^ 0xCA00;
                let rating = clamp(lvl - 4.0 + 4.0 * key_normal(0xF112 ^ year as u64, key), lo - 6.0, hi);
                let age = 22 + (key_uniform(0xA6F ^ year as u64, key) * 7.0) as u32;
                let seed = (w.options.seed ^ 0x0CA1 ^ (year as u32) << 6) ^ (fr as u32 * 983 + got as u32 * 37);
                let school = national_school(key ^ (year as u64) << 20);
                let i = new_player(w, pos, rating, age, seed, fr, Some(school));
                sign(w, i, k, fr);
                roster.push(i as u32);
                got += 1;
                nat += 1;
            }
            have.insert(pos, got);
        }
    }
    for &(pos, n) in &need {
        let mut got = have.get(&pos).copied().unwrap_or(0);
        while got < n {
            let next = pool.get_mut(&pos).and_then(|v| if v.is_empty() { None } else { Some(v.remove(0)) });
            let i = match next {
                Some((i, _)) => i,
                None => {
                    let rating = clamp(
                        lvl - 3.0 + 4.0 * key_normal(0xF111 ^ year as u64, (fr * 100 + got) as u64),
                        lo - 6.0,
                        hi,
                    );
                    let age = 23 + (key_uniform(0xA6E ^ year as u64, (fr * 100 + got) as u64) * 6.0) as u32;
                    let seed = (w.options.seed ^ 0x0AF1 ^ (year as u32) << 6) ^ (fr as u32 * 977 + got as u32 * 31);
                    new_player(w, pos, rating, age, seed, fr, None)
                }
            };
            sign(w, i, k, fr);
            roster.push(i as u32);
            got += 1;
        }
    }
    // Camp cuts: a spot with more players than places lets the weakest go
    // (its holdovers and draft picks can run over), keeping the Canadians
    // the ratio needs.
    let want = if k == CFL { ratio(year).0 } else { 0 };
    let is_nat = |w: &World, i: u32| k == CFL && w.others.national(w.people[i as usize].player.id);
    let mut nats = roster.iter().filter(|&&i| is_nat(w, i)).count();
    for &(pos, n) in &need {
        let mut here: Vec<(f64, u32)> = roster
            .iter()
            .copied()
            .filter(|&i| w.people[i as usize].player.position == pos)
            .map(|i| (w.people[i as usize].player.overall(), i))
            .collect();
        here.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut over = here.len().saturating_sub(n);
        for &(_, i) in &here {
            if over == 0 {
                break;
            }
            let nat = is_nat(w, i);
            if nat && nats <= want {
                continue;
            }
            nats -= nat as usize;
            roster.retain(|&x| x != i);
            release(w, i as usize, k);
            over -= 1;
        }
    }
    w.others.teams[fr].roster = roster;
}

/// A player signs with a team in another league.
fn sign(w: &mut World, i: usize, k: u8, fr: usize) {
    let year = w.year;
    let p = &mut w.people[i];
    if let Some(c) = p.pro.as_mut() {
        if c.status == Status::Other(k) {
            return;
        }
        c.status = Status::Other(k);
        c.weeks_unsigned = 0;
    }
    let age = p.row_age();
    p.note(format!("Signed with the {} ({}) at {age}.", name(fr, year), league_short(k)));
}

/// A player from outside the world's tracked classes: from a small college,
/// a U Sports program or abroad. Built like the market's veterans.
/// `national`: a Canadian, with the school he came from (`U_SPORTS`).
#[allow(clippy::too_many_arguments)]
fn new_player(
    w: &mut World,
    pos: Position,
    rating: f64,
    age: u32,
    seed: u32,
    fr: usize,
    national: Option<u8>,
) -> usize {
    let year = w.year;
    let mut p = super::veteran(w, pos, rating, age, seed);
    p.notes.clear();
    let from = match national {
        Some(s) => U_SPORTS[s as usize % U_SPORTS.len()].to_string(),
        None => "a small college".to_string(),
    };
    p.college = None;
    p.path.clear();
    p.note(format!("Came to the {} from {from} at {age} (generated player).", name(fr, year)));
    p.pro = Some(Box::new(super::ProCareer::new(None, Status::FreeAgent, year)));
    let id = p.player.id;
    w.people.push(p);
    if let Some(s) = national {
        w.others.nationals.insert(id, s);
    }
    w.people.len() - 1
}

/// A U Sports school for a generated Canadian (three in ten from an
/// American college), keyed so it never changes.
fn national_school(key: u64) -> u8 {
    let u = key_uniform(0x5C40, key);
    if u < 0.3 { (U_SPORTS.len() - 1) as u8 } else { ((u - 0.3) / 0.7 * (U_SPORTS.len() - 1) as f64) as u8 }
}

/// The CFL's ratio (Claude's reading of the collective agreements): 20
/// Canadians on a 42-man roster to 2013, 21 of 46 from 2014, and seven of
/// them starting.
pub fn ratio(year: i32) -> (usize, usize) {
    (if year >= 2014 { 21 } else { 20 }, 7)
}

/// Where a generated Canadian grew up: a town in his school's province.
pub fn national_home(id: u32, school: u8) -> String {
    const PROV: [(&str, &[&str]); 7] = [
        (
            "ON",
            &[
                "Toronto",
                "Mississauga",
                "Brampton",
                "Ottawa",
                "Hamilton",
                "London",
                "Windsor",
                "Oakville",
                "Kitchener",
                "Guelph",
                "Burlington",
                "Kingston",
            ],
        ),
        ("QC", &["Montreal", "Quebec City", "Laval", "Longueuil", "Gatineau", "Sherbrooke", "Terrebonne"]),
        ("BC", &["Vancouver", "Surrey", "Burnaby", "Kelowna", "Victoria", "Langley"]),
        ("AB", &["Calgary", "Edmonton", "Red Deer", "Lethbridge"]),
        ("SK", &["Regina", "Saskatoon", "Moose Jaw"]),
        ("MB", &["Winnipeg", "Brandon", "Steinbach"]),
        ("NS", &["Halifax", "Dartmouth", "Sydney"]),
    ];
    // Each school's province (the American colleges' Canadians from all over,
    // by where the country's players come from).
    let prov = match U_SPORTS.get(school as usize).copied().unwrap_or("") {
        "Laval" | "Montréal" | "Bishop's" | "Concordia" | "McGill" | "Sherbrooke" => 1,
        "UBC" => 2,
        "Calgary" | "Alberta" => 3,
        "Saskatchewan" | "Regina" => 4,
        "Manitoba" => 5,
        "Saint Mary's" => 6,
        "an American college" => {
            let u = key_uniform(0x9A0F, id as u64);
            [0.4, 0.6, 0.72, 0.84, 0.9, 0.96, 1.0].iter().position(|&c| u < c).unwrap_or(0)
        }
        _ => 0,
    };
    let (code, towns) = PROV[prov];
    let t = towns[(key_uniform(0x70E1, id as u64) * towns.len() as f64) as usize % towns.len()];
    format!("{t}, {code}")
}

/// The CFL draft (early May): Canadians from U Sports programs and
/// American colleges, eight rounds, worst team first. Real: about 72
/// picks a year from 2016 (nine teams, eight rounds), the best ready to
/// play, most special teams and backups (proposed, not confirmed: grades
/// 44-60).
fn draft(w: &mut World, d: Date) {
    let year = d.year();
    let teams = teams_in(CFL, year);
    if teams.is_empty() {
        return;
    }
    // Worst record last season first.
    let last = w.others.seasons.iter().rev().find(|s| s.league == CFL);
    let mut order: Vec<usize> = teams.clone();
    order.sort_by(|&a, &b| {
        let r = |fr: usize| last.and_then(|s| s.table.iter().find(|r| r.fr as usize == fr)).map_or(0.5, |r| r.pct());
        r(a).total_cmp(&r(b)).then(a.cmp(&b))
    });
    let rounds = 8;
    let n = rounds * order.len();
    use Position::*;
    let mix = [
        (IOL, 12.0),
        (OT, 8.0),
        (DT, 7.0),
        (EDGE, 6.0),
        (LB, 15.0),
        (S, 10.0),
        (CB, 9.0),
        (WR, 14.0),
        (RB, 8.0),
        (TE, 2.0),
        (K, 2.0),
        (P, 2.0),
        (QB, 2.0),
    ];
    let mut rng = Rng::new(w.options.seed ^ 0xCF1D ^ year as u32);
    let mut prospects: Vec<(Position, f64, u32, u8)> = (0..n)
        .map(|r| {
            let pos = rng.weighted(&mix.map(|(p, x)| (p, x)));
            let rating = clamp(58.0 - 0.17 * r as f64 + rng.normal(0.0, 2.0), 40.0, 62.0);
            let age = rng.int(21, 24) as u32;
            let school =
                if rng.chance(0.3) { U_SPORTS.len() - 1 } else { rng.int(0, U_SPORTS.len() as i32 - 2) as usize };
            (pos, rating, age, school as u8)
        })
        .collect();
    prospects.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut pick = 0u16;
    for _ in 0..rounds {
        for &fr in &order {
            if prospects.is_empty() {
                break;
            }
            // The best left, a little toward a need.
            let have = |pos: Position, w: &World| {
                w.others.teams[fr].roster.iter().filter(|&&i| w.people[i as usize].player.position == pos).count()
            };
            let best = (0..prospects.len().min(6))
                .max_by(|&a, &b| {
                    let s = |j: usize| prospects[j].1 - 0.8 * have(prospects[j].0, w) as f64;
                    s(a).total_cmp(&s(b))
                })
                .unwrap_or(0);
            let (pos, rating, age, school) = prospects.remove(best);
            pick += 1;
            let seed = rng.next_u32();
            let i = new_player(w, pos, rating, age, seed, fr, Some(school));
            let round = (pick - 1) / order.len() as u16 + 1;
            let from = U_SPORTS[school as usize];
            let p = &mut w.people[i];
            p.notes.clear();
            let age_now = p.row_age();
            p.note(format!(
                "Drafted by the {} in round {round} of the {year} CFL draft (pick {pick}) out of {from}, at {age_now}.",
                name(fr, year)
            ));
            if let Some(c) = p.pro.as_mut() {
                c.status = Status::Other(CFL);
            }
            w.others.teams[fr].roster.push(i as u32);
            let (id, name) = (w.people[i].player.id, w.people[i].name.clone());
            w.others.draft.push(Pick { year, pick, team: fr as u16, id, name, pos, school });
        }
    }
    if w.recording {
        let text = format!("The {year} CFL draft: {pick} Canadians picked over {rounds} rounds.");
        super::market::post(w, d, None, None, Loudness::Small, Kind::News, text);
    }
}

/// One team ready for a game: the healthy roster, its staff calling plays.
struct Side {
    team: GameTeam,
    who: HashMap<u32, usize>,
}

fn caller(w: &World, c: Option<usize>, fallback: Option<usize>) -> Caller {
    let Some(c) = c.or(fallback) else { return Caller::average() };
    let co = &w.coaches[c];
    let a = co.ability();
    let id = c as u64;
    let base = 0.6 * a.scheme + 0.4 * a.game_management;
    Caller {
        skill: base,
        adaptation: a.game_management,
        aggressiveness: co.personality.ambition,
        reading: clamp(0.7 * a.scheme + 0.3 * a.game_management + 2.0 * key_normal(0x5EAD, id), 1.0, 20.0),
        situational: clamp(0.7 * a.game_management + 0.3 * a.scheme + 2.0 * key_normal(0x517A, id), 1.0, 20.0),
        creativity: clamp(a.scheme + 3.0 * key_normal(0xC4EA, id), 1.0, 20.0),
        nerve: clamp(0.04 * (co.personality.temper - 10.0), -1.0, 1.0),
    }
}

fn side(w: &World, k: u8, fr: usize, year: i32) -> Option<Side> {
    let team = &w.others.teams[fr];
    let mut who = HashMap::new();
    let mut players = Vec::new();
    let mut nationals: HashSet<u32> = HashSet::new();
    // A spring league's teams meet in a three-week camp, most of them new
    // to each other and to the playbook: they know less of it than a club
    // whose players come back (proposed, not confirmed: 0.9 of the CFL's
    // and arena football's knowledge, which keep their players year on
    // year).
    let camp = if matches!(k, CFL | AFL1 | AFL2) { 1.0 } else { 0.9 };
    let mut line_back = 0;
    for &i in &team.roster {
        let p = &w.people[i as usize];
        let Some(c) = p.pro.as_ref() else { continue };
        if c.out_weeks > 0 || p.stage != Stage::Pro {
            continue;
        }
        // Seasons in this playbook, as the NFL counts them (`pro::games`):
        // with this team, this one counted, read through his football mind.
        let before = c.other.iter().filter(|s| s.team as usize == fr && s.year < year).count();
        let here = before as f64 + 1.0;
        let iq = 0.5
            * (p.player.attributes.get(crate::attributes::Attr::FootballIq)
                + p.player.attributes.get(crate::attributes::Attr::PlayRecognition));
        let mind = clamp(0.92 + 0.08 * (iq - 60.0) / 25.0, 0.82, 1.04);
        let know = clamp(0.82 + 0.05 * here, 0.0, 0.97) * 0.92 * camp * mind;
        // Linemen who started up front here last season (the NFL's rule).
        if matches!(p.player.position, Position::OT | Position::IOL)
            && c.other
                .iter()
                .any(|s| s.team as usize == fr && s.year == year - 1 && s.games > 0 && 2 * s.starts >= s.games)
        {
            line_back += 1;
        }
        let experience = p.college_snaps as f64 / crate::world::season::SEASON_SNAPS as f64
            + c.seasons.iter().map(|s| (s.snaps[0] + s.snaps[1]) as f64).sum::<f64>() / 1000.0
            + c.other.iter().map(|s| s.games as f64).sum::<f64>() / 18.0;
        who.insert(p.player.id, i as usize);
        if k == CFL && w.others.national(p.player.id) {
            nationals.insert(p.player.id);
        }
        players.push(GamePlayer::from_player(&p.player, know, experience));
    }
    let min = if matches!(k, AFL1 | AFL2) { 16 } else { 30 };
    if players.len() < min {
        return None;
    }
    let hc = team.head_coach;
    let scheme = if matches!(k, AFL1 | AFL2 | CFL) { OffScheme::AirRaid } else { OffScheme::WestCoast };
    let mut staff = GameStaff::average(scheme);
    staff.offense = caller(w, team.oc, hc);
    staff.defense = caller(w, team.dc, hc);
    if let Some(h) = hc {
        let a = w.coaches[h].ability();
        staff.game_management = a.game_management;
        staff.strictness = a.discipline;
        staff.sportsmanship = w.coaches[h].sportsmanship;
        staff.style = w.coaches[h].style();
    }
    staff.year = year;
    let mut t = GameTeam::new(name(fr, year).to_string(), players, staff);
    t.give_orders(crate::player::Level::NFL);
    // A line that played together last year (the NFL's rule); a new
    // team's line has just met.
    t.line_chem = 0.25 + 0.15 * line_back.min(5) as f64;
    // Your club: your depth chart where your job sets it (an NFL seat's
    // rule), the CFL's ratio met the cheapest way from your order.
    let yours = w.seat.is_some_and(|s| s.level == crate::calendar::Level::Other && s.other == Some(fr));
    let mine = if yours { w.seat_order() } else { Vec::new() };
    if k == CFL {
        let fixed = ratio_starters_from(&t.players, &nationals, ratio(year).1, &mine);
        let set: Vec<Position> = fixed.iter().map(|x| x.0).collect();
        t.order = mine.into_iter().filter(|(p, _)| !set.contains(p)).chain(fixed).collect();
    } else {
        t.order = mine;
    }
    Some(Side { team: t, who })
}

/// Your club as the engine would dress it today.
pub(crate) fn seat_side(w: &World, fr: usize) -> Option<GameTeam> {
    Some(side(w, league_of(fr), fr, w.today.year())?.team)
}

/// The seed of a league's week (each game mixes in its place in the week).
fn week_seed(w: &World, k: u8, year: i32, wk: usize) -> u64 {
    w.options.seed as u64 ^ (year as u64) << 24 ^ (k as u64) << 16 ^ (wk as u64) << 8
}

/// The games a league plays on its next week: the regular season's week,
/// or the playoff round (seeded from the table first when the regular
/// season just ended). Empty when the season is over or stopped.
fn next_pairs(live: &Live, f: &Format) -> Vec<(u16, u16)> {
    let wk = live.week as usize;
    let reg = live.weeks.len();
    let stopped = f.stop_after.is_some_and(|s| wk >= s as usize);
    if wk < reg && !stopped {
        return live.weeks[wk].clone();
    }
    if wk == reg || stopped && live.round == 0 {
        if stopped || f.playoff == Playoff::None {
            return Vec::new();
        }
        let mut c = live.clone();
        seed_playoffs(&mut c, f);
        return playoff_pairs(&c, f);
    }
    playoff_pairs(live, f)
}

/// Your club's league season under way: its place in `others.live` and
/// its format.
fn seat_league(w: &World) -> Option<(usize, usize, Format)> {
    let s = w.seat?;
    let fr = s.other.filter(|_| s.level == crate::calendar::Level::Other)?;
    let k = league_of(fr);
    let year = w.today.year();
    let f = format(k, year)?;
    let li = w.others.live.iter().position(|l| l.league == k && l.year == year && !l.done)?;
    Some((fr, li, f))
}

/// The games left for your club this season, by date: the regular season's
/// weeks still to play, then, once the playoffs start, the next round
/// while your club is alive. (week date, home, away)
pub fn seat_fixtures(w: &World) -> Vec<(Date, usize, usize)> {
    let Some((fr, li, f)) = seat_league(w) else { return Vec::new() };
    let live = &w.others.live[li];
    let mut out = Vec::new();
    let reg = live.weeks.len();
    let stop = f.stop_after.map_or(reg, |s| (s as usize).min(reg));
    let from = live.week as usize;
    for wk in from..stop {
        if let Some(&(h, a)) = live.weeks[wk].iter().find(|&&(h, a)| h as usize == fr || a as usize == fr) {
            out.push((f.start.add(7 * wk as i32), h as usize, a as usize));
        }
    }
    if from >= stop {
        let d = f.start.add(7 * from as i32);
        for (h, a) in next_pairs(live, &f) {
            if h as usize == fr || a as usize == fr {
                out.push((d, h as usize, a as usize));
            }
        }
    }
    out
}

/// The teams your club may still meet: the rest of its schedule, and in
/// the playoffs every team still alive.
pub fn seat_opponents(w: &World) -> Vec<usize> {
    let Some((fr, li, f)) = seat_league(w) else { return Vec::new() };
    let mut v: Vec<usize> = seat_fixtures(w).into_iter().map(|(_, h, a)| if h == fr { a } else { h }).collect();
    let live = &w.others.live[li];
    let reg = f.stop_after.map_or(live.weeks.len(), |s| (s as usize).min(live.weeks.len()));
    if live.week as usize >= reg && live.alive.contains(&(fr as u16)) {
        v.extend(live.alive.iter().map(|&x| x as usize).filter(|&x| x != fr));
    }
    v.sort_unstable();
    v.dedup();
    v
}

/// Your club's game today as the day will play it.
pub(crate) fn seat_setup(w: &World) -> Option<crate::world::live_seat::SeatSetup> {
    let (fr, li, f) = seat_league(w)?;
    let live = &w.others.live[li];
    let wk = live.week as usize;
    if w.today != f.start.add(7 * wk as i32) {
        return None;
    }
    let pairs = next_pairs(live, &f);
    let g = pairs.iter().position(|&(h, a)| h as usize == fr || a as usize == fr)?;
    let (h, a) = pairs[g];
    let (k, year) = (live.league, live.year);
    let sh = side(w, k, h as usize, year)?;
    let sa = side(w, k, a as usize, year)?;
    let round =
        if wk < live.weeks.len() && f.stop_after.is_none_or(|s| wk < s as usize) { 0 } else { live.round.max(1) };
    // The title game's neutral site, as `play_week` reads it after seeding.
    let mut after = live.clone();
    if round > 0 && after.round == 0 {
        seed_playoffs(&mut after, &f);
    }
    let neutral = round > 0 && last_round(&f, &after);
    let mut setup = crate::game::GameSetup::new(
        &sh.team,
        &sa.team,
        crate::player::Level::NFL,
        week_seed(w, k, year, wk) ^ (g as u64 * 7919),
    );
    // As `quick_game_book` sets it up, so the day plays the same game.
    let v = Venue { neutral, ..Venue::default() };
    setup.rules = crate::game::Rules::for_book(book(k), year);
    setup.year = year;
    setup.playoff = round > 0;
    setup.cond = v.cond;
    setup.neutral = v.neutral;
    setup.wear = v.wear;
    setup.cold_unused = v.cold_unused;
    let params = crate::game::live::Params::of(&setup);
    Some(crate::world::live_seat::SeatSetup {
        level: crate::calendar::Level::Other,
        teams: [h as usize, a as usize],
        names: [name(h as usize, year).to_string(), name(a as usize, year).to_string()],
        home: sh.team,
        away: sa.team,
        params,
    })
}

/// The CFL's starters by position, twelve a side: five on the offensive
/// line, a quarterback, a back, a tight end and four receivers; four on the
/// defensive line, two linebackers, six in the secondary.
pub(crate) fn cfl_starts(pos: Position) -> usize {
    use Position::*;
    match pos {
        QB | RB | TE => 1,
        WR => 4,
        OT => 2,
        IOL => 3,
        EDGE | DT | LB => 2,
        CB => 4,
        S => 2,
        K | P | LS => 0,
    }
}

/// The CFL's starting ratio: when fewer Canadians than the rule asks would
/// start on merit, the coach starts the Canadians who cost him least, each
/// in place of the weakest import at his spot. The order is the staff's
/// own depth chart for every spot it changes.
#[cfg(test)]
pub(crate) fn ratio_starters(ps: &[GamePlayer], nationals: &HashSet<u32>, need: usize) -> Vec<(Position, Vec<u32>)> {
    ratio_starters_from(ps, nationals, need, &[])
}

/// `ratio_starters` from a coach's own order where he set one (your
/// depth chart): his men stay where he put them, and only the spots the
/// ratio needs change.
pub(crate) fn ratio_starters_from(
    ps: &[GamePlayer],
    nationals: &HashSet<u32>,
    need: usize,
    order: &[(Position, Vec<u32>)],
) -> Vec<(Position, Vec<u32>)> {
    let starts = cfl_starts;
    let mut lists: Vec<(Position, Vec<(f64, u32)>)> = Position::ALL
        .iter()
        .filter(|&&pos| starts(pos) > 0)
        .map(|&pos| {
            let mut v: Vec<(f64, u32)> =
                ps.iter().filter(|p| p.position == pos).map(|p| (p.overall_at(pos), p.id)).collect();
            v.sort_by(|a, b| b.0.total_cmp(&a.0));
            if let Some((_, ids)) = order.iter().find(|(p, _)| *p == pos) {
                // His listed men, then the rest as the staff ranks them.
                let mut listed: Vec<(f64, u32)> = ids
                    .iter()
                    .filter_map(|&id| ps.iter().find(|p| p.id == id).map(|p| (p.overall_at(pos), id)))
                    .collect();
                for x in v.iter() {
                    if !listed.iter().any(|y| y.1 == x.1) {
                        listed.push(*x);
                    }
                }
                v = listed;
            }
            (pos, v)
        })
        .collect();
    let starting = |lists: &[(Position, Vec<(f64, u32)>)]| {
        lists
            .iter()
            .map(|(pos, v)| v.iter().take(starts(*pos)).filter(|x| nationals.contains(&x.1)).count())
            .sum::<usize>()
    };
    let mut changed: HashSet<usize> = HashSet::new();
    while starting(&lists) < need {
        // The cheapest swap: the best Canadian on the bench for the weakest
        // import starting at his spot.
        let mut best: Option<(f64, usize, usize, usize)> = None;
        for (li, (pos, v)) in lists.iter().enumerate() {
            let n = starts(*pos).min(v.len());
            let Some(out) = (0..n).rev().find(|&j| !nationals.contains(&v[j].1)) else { continue };
            let Some(inn) = (n..v.len()).find(|&j| nationals.contains(&v[j].1)) else { continue };
            let cost = v[out].0 - v[inn].0;
            if best.is_none_or(|b| cost < b.0) {
                best = Some((cost, li, out, inn));
            }
        }
        let Some((_, li, out, inn)) = best else { break };
        lists[li].1.swap(out, inn);
        changed.insert(li);
    }
    let mut changed: Vec<usize> = changed.into_iter().collect();
    changed.sort_unstable();
    changed.into_iter().map(|li| (lists[li].0, lists[li].1.iter().map(|x| x.1).collect())).collect()
}

/// A week of games (or a playoff round): each on the engine under the
/// league's rule book, all cores; the lines go on each player's page.
fn play_week(w: &mut World, d: Date, li: usize, f: &Format) {
    let (k, year) = (w.others.live[li].league, w.others.live[li].year);
    let wk = w.others.live[li].week as usize;
    let reg = w.others.live[li].weeks.len();
    let stopped = f.stop_after.is_some_and(|s| wk >= s as usize);
    let fixtures: Vec<(u16, u16)> = if wk < reg && !stopped {
        w.others.live[li].weeks[wk].clone()
    } else {
        if wk == reg || stopped && w.others.live[li].round == 0 {
            if stopped || f.playoff == Playoff::None {
                finish(w, d, li, f, if stopped { "the league stopped play" } else { "" });
                return;
            }
            seed_playoffs(&mut w.others.live[li], f);
        }
        playoff_pairs(&w.others.live[li], f)
    };
    let round = w.others.live[li].round;
    if fixtures.is_empty() {
        finish(w, d, li, f, "");
        return;
    }
    let mut sides: HashMap<u16, Option<Side>> = HashMap::new();
    for &(h, a) in &fixtures {
        for t in [h, a] {
            sides.entry(t).or_insert_with(|| side(w, k, t as usize, year));
        }
    }
    let bk = book(k);
    let seed = week_seed(w, k, year, wk);
    let jobs: Vec<(usize, u16, u16)> = fixtures.iter().enumerate().map(|(g, &(h, a))| (g, h, a)).collect();
    // Your game, played live on the game day screens (later extras topic
    // 5: a seat here works like an NFL seat under the league's rules).
    let live = w.live_result(crate::calendar::Level::Other);
    // Your game as the morning built it (`seat_setup`), so the game your
    // staff coaches is the one the game day screen showed.
    let yours = seat_setup(w);
    let neutral = round > 0 && last_round(f, &w.others.live[li]);
    let results: Vec<Option<GameResult>> = jobs
        .par_iter()
        .map(|&(g, h, a)| {
            let (Some(Some(sh)), Some(Some(sa))) = (sides.get(&h), sides.get(&a)) else { return None };
            if let Some((t, r)) = &live
                && *t == [h as usize, a as usize]
            {
                return Some(r.clone());
            }
            if let Some(m) = &yours
                && m.teams == [h as usize, a as usize]
            {
                // Kept with its setup, so its film plays again in the film room.
                let mut setup = m.params.setup(&m.home, &m.away);
                setup.keep = true;
                return Some(crate::game::play_game(&setup));
            }
            // A title game is at a neutral site (the Grey Cup's host city).
            let venue = Venue { neutral, ..Venue::default() };
            Some(crate::game::quick_game_book(&sh.team, &sa.team, bk, year, seed ^ (g as u64 * 7919), venue, round > 0))
        })
        .collect();
    for (g, r) in results.into_iter().enumerate() {
        let (h, a) = fixtures[g];
        // A team that can't dress a side forfeits (never in practice: rosters
        // are filled before the season).
        let score = match &r {
            Some(r) => [r.stats.team[0].points as u16, r.stats.team[1].points as u16],
            None => [if sides.get(&h).is_some_and(|s| s.is_some()) { 1 } else { 0 }, 0],
        };
        let live = &mut w.others.live[li];
        live.games.push(Game { week: wk as u8, home: h, away: a, score, round });
        if round == 0 {
            for (t, us, them) in [(h, score[0], score[1]), (a, score[1], score[0])] {
                if let Some(row) = live.table.iter_mut().find(|r| r.fr == t) {
                    row.pf += us;
                    row.pa += them;
                    match us.cmp(&them) {
                        std::cmp::Ordering::Greater => row.w += 1,
                        std::cmp::Ordering::Less => row.l += 1,
                        std::cmp::Ordering::Equal => row.t += 1,
                    }
                }
            }
        } else {
            let loser = if score[0] >= score[1] { a } else { h };
            live.alive.retain(|&x| x != loser);
        }
        if let Some(r) = r {
            let (Some(Some(sh)), Some(Some(sa))) = (sides.get(&h), sides.get(&a)) else { continue };
            record(w, k, [h, a], &r, [&sh.who, &sa.who]);
            let teams = [h as usize, a as usize];
            w.keep_seat_game(crate::calendar::Level::Other, teams, &r.stats, r.replay.as_deref());
            w.seat_line(crate::calendar::Level::Other, teams, None, neutral);
            crate::world::stories::other_game(w, teams, &r.stats, round > 0);
            if let Some(o) = w.tape_watch(crate::calendar::Level::Other) {
                w.keep_tape(&o, crate::calendar::Level::Other, teams, &r.stats, &r.film);
            }
        }
    }
    let live = &mut w.others.live[li];
    live.week += 1;
    if round > 0 {
        live.round += 1;
        if live.alive.len() <= 1 {
            finish(w, d, li, f, "");
        }
    }
}

fn last_round(f: &Format, live: &Live) -> bool {
    match f.playoff {
        Playoff::Cfl | Playoff::PerDiv => live.alive.len() == 2 && live.round >= 2,
        _ => live.alive.len() == 2,
    }
}

/// The playoff field from the standings.
fn seed_playoffs(live: &mut Live, f: &Format) {
    let year = live.year;
    let rank = |rows: &mut Vec<Row>| {
        rows.sort_by(|a, b| {
            b.points()
                .cmp(&a.points())
                .then(b.w.cmp(&a.w))
                .then((b.pf as i32 - b.pa as i32).cmp(&(a.pf as i32 - a.pa as i32)))
        })
    };
    let div = |fr: u16| def(fr as usize, year).map_or(0, |d| d.div);
    let mut alive = Vec::new();
    match f.playoff {
        Playoff::Cfl => {
            let mut west: Vec<Row> = live.table.iter().copied().filter(|r| div(r.fr) == 0).collect();
            let mut east: Vec<Row> = live.table.iter().copied().filter(|r| div(r.fr) == 1).collect();
            rank(&mut west);
            rank(&mut east);
            // The crossover: a fourth-place team with more points than the
            // other division's third takes its place.
            let mut w3: Vec<Row> = west.iter().take(3).copied().collect();
            let mut e3: Vec<Row> = east.iter().take(3).copied().collect();
            if let (Some(w4), Some(e3l)) = (west.get(3), e3.last())
                && w4.points() > e3l.points()
            {
                e3.pop();
                e3.push(*w4);
            } else if let (Some(e4), Some(w3l)) = (east.get(3), w3.last())
                && e4.points() > w3l.points()
            {
                w3.pop();
                w3.push(*e4);
            }
            // Each division: 1, then 2 hosting 3.
            alive.extend(w3.iter().map(|r| r.fr));
            alive.extend(e3.iter().map(|r| r.fr));
        }
        Playoff::Top(n) => {
            let mut rows = live.table.clone();
            rank(&mut rows);
            alive = rows.iter().take(n as usize).map(|r| r.fr).collect();
        }
        Playoff::PerDiv => {
            for dv in 0..2 {
                let mut rows: Vec<Row> = live.table.iter().copied().filter(|r| div(r.fr) == dv).collect();
                rank(&mut rows);
                alive.extend(rows.iter().take(2).map(|r| r.fr));
            }
        }
        Playoff::None => {}
    }
    live.alive = alive;
    live.round = 1;
}

/// This round's games from the teams still alive.
fn playoff_pairs(live: &Live, f: &Format) -> Vec<(u16, u16)> {
    let a = &live.alive;
    match f.playoff {
        Playoff::Cfl => match live.round {
            // The semi-finals: 2 hosts 3 in each division.
            1 if a.len() >= 6 => vec![(a[1], a[2]), (a[4], a[5])],
            // The finals: 1 hosts the winner.
            2 if a.len() >= 4 => vec![(a[0], a[1]), (a[2], a[3])],
            _ if a.len() >= 2 => vec![(a[0], a[1])],
            _ => Vec::new(),
        },
        Playoff::PerDiv => match live.round {
            1 if a.len() >= 4 => vec![(a[0], a[1]), (a[2], a[3])],
            _ if a.len() >= 2 => vec![(a[0], a[1])],
            _ => Vec::new(),
        },
        _ => (0..a.len() / 2).map(|i| (a[i], a[a.len() - 1 - i])).collect(),
    }
}

/// A game's lines on each player's page, and its injuries.
fn record(w: &mut World, k: u8, teams: [u16; 2], r: &GameResult, who: [&HashMap<u32, usize>; 2]) {
    let year = w.year;
    for side in 0..2 {
        let max_off = r.stats.players[side].values().map(|l| l.snaps_off).max().unwrap_or(1).max(1);
        let max_def = r.stats.players[side].values().map(|l| l.snaps_def).max().unwrap_or(1).max(1);
        for (&id, l) in &r.stats.players[side] {
            let Some(&i) = who[side].get(&id) else { continue };
            let Some(c) = w.people[i].pro.as_mut() else { continue };
            let dressed = l.snaps_off + l.snaps_def + l.snaps_st > 0;
            if !dressed {
                continue;
            }
            let starter = 2 * l.snaps_off >= max_off || 2 * l.snaps_def >= max_def;
            let fr = teams[side];
            let s = match c.other.iter().position(|s| s.year == year && s.team == fr) {
                Some(j) => &mut c.other[j],
                None => {
                    c.other.push(OtherSeason { year, league: k, team: fr, ..OtherSeason::default() });
                    c.other.last_mut().unwrap()
                }
            };
            s.games += 1;
            s.starts += starter as u16;
            s.cmp += l.cmp as u16;
            s.att += l.att as u16;
            s.pass_yds += l.pass_yds as f32;
            s.pass_td += l.pass_td as u16;
            s.int += l.int as u16;
            s.rush += l.rush as u16;
            s.rush_yds += l.rush_yds as f32;
            s.rush_td += l.rush_td as u16;
            s.rec += l.rec as u16;
            s.rec_yds += l.rec_yds as f32;
            s.rec_td += l.rec_td as u16;
            s.tackles += (l.tackles + l.assists) as u16;
            s.sacks += l.sacks as f32;
            s.ints += l.ints as u16;
            s.fg += l.fg_made.iter().sum::<u32>() as u16;
            s.fg_att += l.fg_att.iter().sum::<u32>() as u16;
            c.sharpness = (c.sharpness + 0.34).min(1.0);
        }
    }
    for (_, inj) in &r.injuries {
        if inj.plays_on || inj.side > 1 {
            continue;
        }
        let Some(&i) = who[inj.side].get(&inj.id) else { continue };
        if let Some(c) = w.people[i].pro.as_mut() {
            c.out_weeks = c.out_weeks.max(inj.weeks.ceil().max(1.0) as u32);
        }
    }
}

/// A season ends: the champion, the record, the pay, the snaps for his
/// development; players go back to the street (a CFL team keeps most of
/// its own under contract); the carousel for the coaches.
fn finish(w: &mut World, d: Date, li: usize, f: &Format, note: &'static str) {
    let live = &mut w.others.live[li];
    live.done = true;
    let (k, year) = (live.league, live.year);
    let champion = if live.round > 0 && live.alive.len() == 1 { Some(live.alive[0]) } else { None };
    let title = live.games.iter().rev().find(|g| g.round > 0).copied().filter(|_| champion.is_some());
    let runner_up = title.map(|g| if Some(g.home) == champion { g.away } else { g.home });
    let final_score =
        title.map_or([0, 0], |g| if Some(g.home) == champion { g.score } else { [g.score[1], g.score[0]] });
    let mut table = live.table.clone();
    table.sort_by(|a, b| b.pct().total_cmp(&a.pct()).then(b.pf.cmp(&a.pf)));
    let games = std::mem::take(&mut live.games);
    let season = Season { league: k, year, table: table.clone(), champion, runner_up, final_score, note, games };
    w.others.live.remove(li);
    // Only the last two seasons keep their games.
    let n = w.others.seasons.len();
    for s in w.others.seasons.iter_mut().take(n.saturating_sub(24)) {
        s.games = Vec::new();
    }
    w.others.seasons.push(season);
    if let Some(c) = champion {
        w.others.teams[c as usize].titles += 1;
        if let Some(hc) = w.others.teams[c as usize].head_coach {
            w.coaches[hc].titles += 1;
        }
    }
    for r in &table {
        let t = &mut w.others.teams[r.fr as usize];
        t.win_share.push(r.pct() as f32);
        if t.win_share.len() > 5 {
            t.win_share.remove(0);
        }
        if let Some(hc) = t.head_coach {
            w.coaches[hc].wins += r.w as u32;
            w.coaches[hc].losses += r.l as u32;
        }
    }
    // Pay, snaps and the street.
    let teams: Vec<usize> = table.iter().map(|r| r.fr as usize).collect();
    for &fr in &teams {
        let roster = w.others.teams[fr].roster.clone();
        let mut keep = Vec::new();
        for i in roster {
            let i = i as usize;
            let star = w.people[i]
                .pro
                .as_ref()
                .and_then(|c| c.other.iter().find(|s| s.year == year && s.team as usize == fr))
                .is_some_and(|s| s.starts as f64 >= 0.7 * f.games as f64);
            let played = w.people[i]
                .pro
                .as_ref()
                .and_then(|c| c.other.iter().find(|s| s.year == year && s.team as usize == fr))
                .map_or(0, |s| s.games);
            let money = pay(k, year, star) * if played > 0 { 1.0 } else { 0.5 };
            if let Some(c) = w.people[i].pro.as_mut() {
                c.earnings += money;
                if let Some(s) = c.other.iter_mut().find(|s| s.year == year && s.team as usize == fr) {
                    s.pay += money as f32;
                }
                // A season of real snaps for his development.
                if played > 0 && k != CFL {
                    c.spring_snaps = c.spring_snaps.max(clamp(played as f64 / 18.0, 0.2, 0.8));
                }
            }
            // A CFL player under contract stays (about two in three each
            // winter; the rest test the market, the best of them the NFL's).
            let age = w.people[i].player.age;
            let stays = k == CFL
                && age <= 33
                && key_uniform(w.options.seed as u64 ^ 0xCF1C ^ (year as u64) << 4, i as u64) < 0.67;
            if stays {
                keep.push(i as u32);
            } else {
                release(w, i, k);
                if age >= 34 && w.people[i].stage == Stage::Pro {
                    super::retire(w, i, d, "end of the season");
                }
            }
        }
        w.others.teams[fr].roster = keep;
    }
    if w.recording {
        let text = match (champion, runner_up) {
            (Some(c), Some(r)) => format!(
                "{} {year}: the {} beat the {} {}-{} in the {}.",
                league_name(k, year),
                name(c as usize, year),
                name(r as usize, year),
                final_score[0],
                final_score[1],
                f.title
            ),
            _ => format!("The {year} {} season ends without a champion: {note}.", league_name(k, year)),
        };
        let loud = if k == CFL { Loudness::Medium } else { Loudness::Small };
        super::market::post(w, d, None, None, loud, Kind::News, text);
    }
    let fr: Vec<usize> = teams_in(k, year + 1);
    if !fr.is_empty() {
        reviews(w, k, &teams);
    }
}

/// The owners' reviews at a season's end (proposed, not confirmed: the
/// pro owners' rule, a short leash in a small league): a head coach under
/// .350 over his last two seasons is fired most of the time, one under
/// .450 now and then; coordinators go with him half the time.
fn reviews(w: &mut World, k: u8, teams: &[usize]) {
    for &fr in teams {
        let t = &w.others.teams[fr];
        let Some(hc) = t.head_coach else { continue };
        let n = t.win_share.len();
        if n == 0 {
            continue;
        }
        let recent = t.win_share[n.saturating_sub(2)..].iter().map(|&x| x as f64).sum::<f64>() / n.min(2) as f64;
        let since = w.coaches[hc].job.map_or(w.year, |j| j.since);
        if w.year - since < 1 {
            continue;
        }
        let p = if recent < 0.35 {
            0.75
        } else if recent < 0.45 {
            0.25
        } else {
            0.0
        };
        if p > 0.0 && key_uniform(w.options.seed as u64 ^ 0xF1E5 ^ (w.year as u64) << 8, hc as u64) < p {
            crate::world::staff::leave(w, hc, "fired");
            if w.recording {
                let text =
                    format!("The {} ({}) fire head coach {}.", name(fr, w.year), league_short(k), w.coaches[hc].name);
                super::market::post(w, w.today, None, None, Loudness::Small, Kind::News, text);
            }
            for role in [Role::OffensiveCoordinator, Role::DefensiveCoordinator] {
                let slot =
                    if role == Role::OffensiveCoordinator { w.others.teams[fr].oc } else { w.others.teams[fr].dc };
                if let Some(a) = slot
                    && key_uniform(0x0C5 ^ w.year as u64, a as u64) < 0.5
                {
                    crate::world::staff::leave(w, a, "let go by the new staff");
                }
            }
        }
    }
}

/// The league's carousel before its camp: every empty seat is filled
/// through the world's coaching market (out-of-work coaches, NFL and
/// college assistants who want a head job, the league's own).
fn carousel(w: &mut World, k: u8, teams: &[usize]) {
    let mut openings = Vec::new();
    // Staffs of franchises that are gone are let go.
    for fr in 0..FRANCHISES {
        if league_of(fr) != k || teams.contains(&fr) {
            continue;
        }
        for c in [w.others.teams[fr].head_coach, w.others.teams[fr].oc, w.others.teams[fr].dc].into_iter().flatten() {
            crate::world::staff::leave(w, c, "his team folded");
        }
    }
    for &fr in teams {
        let t = &w.others.teams[fr];
        for (role, slot) in
            [(Role::HeadCoach, t.head_coach), (Role::OffensiveCoordinator, t.oc), (Role::DefensiveCoordinator, t.dc)]
        {
            if slot.is_none() {
                openings.push((Employer::Other(fr), role));
            }
        }
    }
    crate::world::staff::fill_others(w, openings);
}

/// How good a league's jobs look to a coach, against the NFL's 1.0 and the
/// biggest college programs' 1.0 (proposed, not confirmed: the CFL like a
/// strong FCS job, the spring leagues a little below, arena football like
/// a small college).
pub fn prestige(fr: usize) -> f64 {
    match league_of(fr) {
        CFL => 0.42,
        AFL1 | AFL2 => 0.22,
        _ => 0.36,
    }
}

/// What a league's coaches are paid a year (proposed, not confirmed, from
/// reported salaries: a CFL head coach about $400,000-800,000 in the 2020s,
/// a spring league's about $300,000-500,000, arena football's about
/// $100,000), head coach, coordinator, position coach.
pub fn salary(fr: usize, role: Role, year: i32) -> f64 {
    let era = clamp(0.5 + 0.02 * (year - 1996) as f64, 0.5, 1.2);
    let r = match league_of(fr) {
        CFL => [550_000.0, 160_000.0, 80_000.0],
        AFL1 | AFL2 => [110_000.0, 55_000.0, 35_000.0],
        _ => [400_000.0, 150_000.0, 70_000.0],
    };
    let x = match role {
        Role::HeadCoach => r[0],
        Role::OffensiveCoordinator | Role::DefensiveCoordinator => r[1],
        Role::PositionCoach(_) => r[2],
    };
    x * era
}

/// A coach takes a seat in another league.
pub(crate) fn seat(w: &mut World, fr: usize, role: Role, coach: usize) {
    w.others.ensure();
    let t = &mut w.others.teams[fr];
    match role {
        Role::HeadCoach => t.head_coach = Some(coach),
        Role::OffensiveCoordinator => t.oc = Some(coach),
        Role::DefensiveCoordinator => t.dc = Some(coach),
        Role::PositionCoach(_) => {}
    }
}

/// A coach leaves his seat.
pub(crate) fn unseat(w: &mut World, fr: usize, coach: usize) {
    if let Some(t) = w.others.teams.get_mut(fr) {
        for slot in [&mut t.head_coach, &mut t.oc, &mut t.dc] {
            if *slot == Some(coach) {
                *slot = None;
            }
        }
    }
}

/// The head coach of a franchise.
pub fn head_coach(w: &World, fr: usize) -> Option<usize> {
    w.others.teams.get(fr).and_then(|t| t.head_coach)
}

/// The coach in a seat.
pub fn slot(w: &World, fr: usize, role: Role) -> Option<usize> {
    let t = w.others.teams.get(fr)?;
    match role {
        Role::HeadCoach => t.head_coach,
        Role::OffensiveCoordinator => t.oc,
        Role::DefensiveCoordinator => t.dc,
        Role::PositionCoach(_) => None,
    }
}

/// A franchise's finished seasons: (year, wins, losses, what it came to).
pub fn team_seasons(w: &World, fr: usize) -> Vec<(i32, u32, u32, String)> {
    let mut out = Vec::new();
    for s in &w.others.seasons {
        let Some(r) = s.table.iter().find(|r| r.fr as usize == fr) else { continue };
        let title = format(s.league, s.year).map_or("title game", |f| f.title);
        let note = if s.champion == Some(r.fr) {
            format!("won the {title}")
        } else if s.runner_up == Some(r.fr) {
            format!("lost the {title}")
        } else {
            String::new()
        };
        out.push((s.year, r.w as u32, r.l as u32, note));
    }
    out
}

/// A franchise's regular-season games this year: (date, opponent, at home).
pub fn fixtures(w: &World, fr: usize) -> Vec<(Date, usize, bool)> {
    let k = league_of(fr);
    let Some(f) = format(k, w.year) else { return Vec::new() };
    let Some(live) = w.others.live.iter().find(|l| l.league == k && l.year == w.year) else { return Vec::new() };
    let me = fr as u16;
    let mut v = Vec::new();
    for (wk, games) in live.weeks.iter().enumerate() {
        if f.stop_after.is_some_and(|s| wk >= s as usize) {
            break;
        }
        for &(h, a) in games {
            if h == me || a == me {
                v.push((f.start.add(7 * wk as i32), if h == me { a } else { h } as usize, h == me));
            }
        }
    }
    v
}

/// One game on a club's schedule: (date, opponent, at home, the score as
/// (its points, theirs) once played, the playoff round or 0).
pub type ClubGame = (Date, usize, bool, Option<[u16; 2]>, u8);

/// A franchise's games this year in order. The next playoff game shows once
/// the round is drawn.
pub fn season_games(w: &World, fr: usize) -> Vec<ClubGame> {
    let k = league_of(fr);
    let Some(f) = format(k, w.year) else { return Vec::new() };
    let Some(live) = w.others.live.iter().find(|l| l.league == k && l.year == w.year) else { return Vec::new() };
    let me = fr as u16;
    let ours = |g: &Game| if g.home == me { g.score } else { [g.score[1], g.score[0]] };
    let mut v = Vec::new();
    let stop = f.stop_after.map_or(live.weeks.len(), |s| (s as usize).min(live.weeks.len()));
    for (wk, games) in live.weeks.iter().enumerate().take(stop) {
        for &(h, a) in games.iter().filter(|&&(h, a)| h == me || a == me) {
            let played =
                live.games.iter().find(|g| g.round == 0 && g.week as usize == wk && g.home == h && g.away == a);
            v.push((f.start.add(7 * wk as i32), if h == me { a } else { h } as usize, h == me, played.map(ours), 0));
        }
    }
    for g in live.games.iter().filter(|g| g.round > 0 && (g.home == me || g.away == me)) {
        let home = g.home == me;
        let opp = if home { g.away } else { g.home };
        v.push((f.start.add(7 * g.week as i32), opp as usize, home, Some(ours(g)), g.round));
    }
    if !live.done && live.week as usize >= stop {
        for (h, a) in next_pairs(live, &f).into_iter().filter(|&(h, a)| h == me || a == me) {
            let d = f.start.add(7 * live.week as i32);
            v.push((d, if h == me { a } else { h } as usize, h == me, None, live.round.max(1)));
        }
    }
    v
}

/// A franchise's line in its league's table: this year's season while it
/// runs or once it's done, else the last one it played.
pub fn record_of(w: &World, fr: usize) -> Option<(i32, Row)> {
    let k = league_of(fr);
    let me = fr as u16;
    let live =
        w.others.live.iter().rev().find(|l| l.league == k && l.year <= w.year && l.table.iter().any(|r| r.fr == me));
    if let Some(l) = live {
        return l.table.iter().find(|r| r.fr == me).map(|r| (l.year, *r));
    }
    w.others
        .seasons
        .iter()
        .rev()
        .find(|s| s.league == k)
        .and_then(|s| s.table.iter().find(|r| r.fr == me).map(|r| (s.year, *r)))
}

/// The league's rule book in plain lines, for its page: what differs from
/// the NFL game.
pub fn rules_words(k: u8, year: i32) -> Vec<String> {
    use crate::game::books::{Rouge, Waggle};
    let r = crate::game::Rules::for_book(book(k), year);
    let c = &r.code;
    let f = &c.field;
    let mut v = Vec::new();
    v.push(format!(
        "{} a side on a {:.0}-yard field, {:.0}-yard end zones, {:.0} yards wide",
        c.players, f.length, f.end_zone, f.width
    ));
    v.push(format!("{} downs to make ten yards", if f.downs == 3 { "Three" } else { "Four" }));
    if f.nets {
        v.push("Rebound nets beside the narrow goalposts: kicks and passes off them stay live".into());
    }
    if f.post_depth < 1.0 {
        v.push("The goalposts stand on the goal line".into());
    }
    if c.line_off > 0.0 {
        v.push(format!("The defense lines up {:.0} yard off the ball", c.line_off));
    }
    match c.waggle {
        Waggle::All => v.push("Unlimited motion: every back may move, receivers run at the line at the snap".into()),
        Waggle::One => v.push("One receiver may be running toward the line at the snap".into()),
        _ => {}
    }
    if !c.fair_catch {
        v.push(format!("No fair catch: the cover men give the returner {:.0} yards", c.halo));
    }
    match c.rouge {
        Rouge::Full => v.push("The rouge: a kick not brought out of the end zone scores a single".into()),
        Rouge::Settled => {
            v.push("The rouge only for a kick that settles in the end zone; one through it comes out to the 40".into())
        }
        _ => {}
    }
    if let Some((yards, pts)) = c.long_field_goal {
        v.push(format!("A field goal from {yards:.0} yards or more is worth {pts}"));
    }
    if c.drop_kick {
        v.push("Drop kicks count one more".into());
    }
    let tries: Vec<String> =
        c.try_plays.iter().enumerate().filter_map(|(i, s)| s.map(|y| format!("{} from the {y:.0}", i + 1))).collect();
    let plays = if tries.is_empty() { String::new() } else { format!("a play for {}", tries.join(", ")) };
    v.push(match c.try_kick {
        None => format!("No kicked extra point: the try is {plays}"),
        Some(y) if plays.is_empty() => format!("The try: a kick from the {y:.0}"),
        Some(y) => format!("The try: a kick from the {y:.0}, or {plays}"),
    });
    if !c.kickoffs {
        v.push(format!("No kickoffs: the ball starts at the receiving team's {:.0}", c.no_kick_start));
    }
    if c.arena_kickoff {
        v.push("Kickoffs off the walls and the nets are live".into());
    }
    if let Some(o) = c.onside_alt {
        v.push(format!("Instead of an onside kick: fourth and {:.0} from its own {:.0}", o.togo, o.from));
    }
    if !c.punting {
        v.push("No punting".into());
    } else if c.punt_touchback_35 {
        v.push("A punt into the end zone or out inside the 35 comes out to the 35".into());
    }
    if c.punt_live {
        v.push("A punt that travels 25 yards is a live ball".into());
    }
    if c.max_rushers < 11 {
        v.push(format!("No more than {} may rush the passer", c.max_rushers));
    }
    if c.man_only {
        v.push("Man coverage only behind the rush".into());
    }
    if c.warning >= 170.0 {
        v.push(format!(
            "A three-minute warning{}",
            if c.late_every_play { ", then the clock stops after every play" } else { "" }
        ));
    } else if c.warning <= 0.0 {
        v.push("No two-minute warning".into());
    }
    if c.running_clock > 0.0 {
        v.push(format!(
            "The clock runs on incompletions and out of bounds but in the last {:.0} seconds of each half",
            c.running_clock
        ));
    }
    if c.timeouts_per_game {
        v.push("Two timeouts a game, not a half".into());
    }
    v.push(format!("{:.0}-minute quarters, a {:.0}-second play clock", r.quarter / 60.0, r.play_clock));
    v
}

/// A division's name in a league's season (an empty name: one table).
pub fn division_name(k: u8, year: i32, div: u8) -> &'static str {
    match (k, div) {
        (CFL, 0) => "West Division",
        (CFL, _) => "East Division",
        (XFL01 | XFL20 | AAF, 0) => "Eastern Division",
        (XFL01 | XFL20 | AAF, _) => "Western Division",
        (XFL23 | USFL22, 0) => "North Division",
        (XFL23 | USFL22, _) => "South Division",
        (UFL24, 0) if year >= 2024 => "USFL Conference",
        (UFL24, _) => "XFL Conference",
        _ => "",
    }
}
