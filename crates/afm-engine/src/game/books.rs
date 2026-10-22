//! The other leagues' rule books (later extras topic 5, final build X7):
//! the CFL, arena football, the World League and NFL Europe, the three
//! XFLs, the AAF, both USFLs and both UFLs, each by season. The engine
//! plays every one of them: only the rules differ, the play comes from the
//! players.
//!
//! What each rule is and where it comes from is in `docs/match-engine.md`,
//! "The other leagues' rule books". Where a league's published rule book
//! could not be checked line by line, the rule is marked there as Claude's
//! reading.

use super::geom::{Field, WIDTH};
use super::rules::{FirstDownStop, Overtime, Rules};
use crate::player::Level;

/// Which rule book a game is played under.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Book {
    /// The NFL, college and high school (their own levels' rules).
    #[default]
    American,
    /// The Canadian Football League.
    Cfl,
    /// Arena football (the Arena Football League, 1987-2008 and 2010-19).
    Arena,
    /// The World League (1991-92) and NFL Europe (1995-2007).
    NflEurope,
    /// The XFL of 2001.
    Xfl2001,
    /// The XFL of 2020 and 2023.
    Xfl,
    /// The Alliance of American Football (2019).
    Aaf,
    /// The USFL of 1983-85.
    Usfl1983,
    /// The USFL of 2022-23.
    Usfl2022,
    /// The UFL of 2009-12.
    Ufl2009,
    /// The UFL from 2024 (the XFL and the USFL merged).
    Ufl,
}

impl Book {
    pub const ALL: [Book; 10] = [
        Book::Cfl,
        Book::Arena,
        Book::NflEurope,
        Book::Xfl2001,
        Book::Xfl,
        Book::Aaf,
        Book::Usfl1983,
        Book::Usfl2022,
        Book::Ufl2009,
        Book::Ufl,
    ];
    /// The name `match_check` takes for it.
    pub fn key(self) -> &'static str {
        match self {
            Book::American => "american",
            Book::Cfl => "cfl",
            Book::Arena => "arena",
            Book::NflEurope => "nfle",
            Book::Xfl2001 => "xfl2001",
            Book::Xfl => "xfl",
            Book::Aaf => "aaf",
            Book::Usfl1983 => "usfl1983",
            Book::Usfl2022 => "usfl",
            Book::Ufl2009 => "ufl2009",
            Book::Ufl => "ufl",
        }
    }
    pub fn from_key(k: &str) -> Option<Book> {
        Book::ALL.into_iter().find(|b| b.key() == k)
    }
    /// A season the book was in use, for checks that don't name one.
    pub fn sample_year(self) -> i32 {
        match self {
            Book::American => 2026,
            Book::Cfl => 2025,
            Book::Arena => 2015,
            Book::NflEurope => 2005,
            Book::Xfl2001 => 2001,
            Book::Xfl => 2020,
            Book::Aaf => 2019,
            Book::Usfl1983 => 1984,
            Book::Usfl2022 => 2022,
            Book::Ufl2009 => 2010,
            Book::Ufl => 2025,
        }
    }
}

/// The rouge (CFL): a single point when a kick goes dead in the receiving
/// team's end zone or through it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rouge {
    #[default]
    None,
    /// Any kick not brought out of the end zone, or out of it, scores one
    /// (the CFL to 2025).
    Full,
    /// From 2026 only a kick that settles in the end zone and is not
    /// brought out scores; one that goes through or out of the end zone
    /// untouched, or a missed field goal off an upright, gives the
    /// receiving team the ball at its 40.
    Settled,
}

/// Players allowed to move toward the line at the snap.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Waggle {
    /// One man in motion, moving across or back (American rules).
    #[default]
    None,
    /// One receiver may be running toward the line at the snap (arena
    /// football, the XFL of 2001).
    One,
    /// Every backfield player may be in motion, and receivers off the line
    /// may run forward (the CFL).
    All,
}

/// An onside kick's replacement (the AAF, the XFL of 2023, the USFL of 2022
/// and the UFL): one play on fourth and `togo` from the kicking team's own
/// `from`, for a team trailing by `behind` or more, or any team in the
/// last `minutes` of the game.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OnsideAlt {
    pub from: f64,
    pub togo: f64,
    pub behind: i32,
    pub minutes: f64,
}

/// The parts of a rule book the American levels don't use. The default is
/// the American game: 11 a side, four downs, kickoffs, a fair catch, a
/// kicked extra point and a two-point try, the two-minute warning.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Code {
    pub book: Book,
    /// The season the rule book is from.
    pub year: i32,
    /// Players a side on the field.
    pub players: u8,
    /// The field: length, width, end zones, downs, goalposts, nets.
    pub field: Field,
    /// Extra yards the defensive line sets off the ball (CFL: one yard).
    pub line_off: f64,
    pub waggle: Waggle,
    /// A punt or kickoff may be fair caught (none in the CFL or the XFL of
    /// 2001: the cover men must give the returner `halo` yards).
    pub fair_catch: bool,
    pub halo: f64,
    pub rouge: Rouge,
    /// Where the receiving team scrimmages after a single, and after a kick
    /// that gives it the ball without one (the CFL from 2026).
    pub single_spot: f64,
    pub no_single_spot: f64,
    /// A field goal from at least this far is worth this many points (NFL
    /// Europe 50 yards and 4, the UFL from 2026 60 yards and 4).
    pub long_field_goal: Option<(f64, i32)>,
    /// Drop kicks count one more (arena: a drop-kicked field goal 4, a
    /// drop-kicked convert 2).
    pub drop_kick: bool,
    /// The try after a touchdown: where a kick is snapped from (yards from
    /// the goal; None: no kick), and where a play for one, two or three
    /// points is snapped from.
    pub try_kick: Option<f64>,
    pub try_plays: [Option<f64>; 3],
    /// Kickoffs at all (the AAF had none: the ball starts at the receiving
    /// team's `no_kick_start`).
    pub kickoffs: bool,
    pub no_kick_start: f64,
    /// Under a set-up kickoff (the NFL from 2025, the XFL and the UFL):
    /// where a kick short of the landing zone is spotted, and where the
    /// cover team lines up (yards from the receiving goal line).
    pub short_kick_spot: f64,
    pub cover_line: f64,
    /// A kickoff out of bounds: where the receiving team takes it.
    pub kick_out_spot: f64,
    /// Kickoffs land on a field with walls (arena): out of bounds the ball
    /// comes to the receiving 20 (or the spot, if better); a kick dead
    /// beyond the end zone, or downed in it, to the 5.
    pub arena_kickoff: bool,
    pub onside_alt: Option<OnsideAlt>,
    /// Punting allowed (arena: never), and the UFL's 2026 ban on punting
    /// from the opponent's side of the 50 outside the last two minutes of
    /// a half.
    pub punting: bool,
    pub no_punt_past_50: bool,
    /// A punt into the end zone or out of bounds inside the receiving 35 is
    /// spotted at the 35 (the XFL of 2020 and 2023, the UFL).
    pub punt_touchback_35: bool,
    /// A punt that travels 25 yards is a live ball the kicking team may
    /// recover (the XFL of 2001).
    pub punt_live: bool,
    /// The most players who may rush the passer (AAF five; arena four:
    /// three linemen and the Mac linebacker).
    pub max_rushers: u8,
    /// Man coverage behind the rush (arena: the narrow field and the
    /// linebackers' box rules make it a man game).
    pub man_only: bool,
    /// The warning before the end of each half, seconds (two minutes; the
    /// CFL's three-minute warning; none in arena), and whether after it the
    /// clock stops after every play (the CFL).
    pub warning: f64,
    pub late_every_play: bool,
    /// The clock keeps running on incompletions and out of bounds except in
    /// the last `running_clock` seconds of each half (arena football).
    pub running_clock: f64,
    /// Timeouts are per game, not per half (the CFL from 2013, two, one of
    /// them usable after the three-minute warning).
    pub timeouts_per_game: bool,
    /// Coaches' challenges a game.
    pub challenges: u8,
    /// Players go both ways (arena football's ironman rule, to 2006).
    pub ironman: bool,
}

impl Default for Code {
    fn default() -> Code {
        Code {
            book: Book::American,
            year: 0,
            players: 11,
            field: Field::AMERICAN,
            line_off: 0.0,
            waggle: Waggle::None,
            fair_catch: true,
            halo: 0.0,
            rouge: Rouge::None,
            single_spot: 35.0,
            no_single_spot: 40.0,
            long_field_goal: None,
            drop_kick: false,
            try_kick: None,
            try_plays: [None; 3],
            kickoffs: true,
            no_kick_start: 25.0,
            short_kick_spot: 40.0,
            cover_line: 40.0,
            kick_out_spot: 40.0,
            arena_kickoff: false,
            onside_alt: None,
            punting: true,
            no_punt_past_50: false,
            punt_touchback_35: false,
            punt_live: false,
            max_rushers: 11,
            man_only: false,
            warning: 120.0,
            late_every_play: false,
            running_clock: 0.0,
            timeouts_per_game: false,
            challenges: 0,
            ironman: false,
        }
    }
}

impl Code {
    /// The American game (every NFL, college and high school game): the
    /// engine takes its old paths, so results per seed don't move.
    pub fn american(&self) -> bool {
        self.book == Book::American
    }
}

/// A pro rule book's base: NFL rules of the era (the kickoff spot, the
/// extra point and overtime as the NFL had them that season).
fn nfl_era(year: i32) -> Rules {
    let mut r = Rules::for_level(Level::NFL);
    r.nfl_kickoff = false;
    r.kickoff_touchback = 20.0;
    // The kickoff spot: the 35 to 1993, the 30 from 1994, the 35 again from
    // 2011 (touchbacks to the 25 from 2016).
    r.kickoff_from = match year {
        ..=1993 => 35.0,
        1994..=2010 => 30.0,
        _ => 35.0,
    };
    if year >= 2016 {
        r.kickoff_touchback = 25.0;
    }
    // The extra point from the 2 until 2015.
    r.xp_from = if year >= 2015 { 15.0 } else { 2.0 };
    r.overtime = if year >= 2017 { Overtime::NflPeriod } else { Overtime::SuddenDeath };
    r
}

impl Rules {
    /// A league's rules in a season.
    pub fn for_book(book: Book, year: i32) -> Rules {
        let mut code = Code { book, year, ..Code::default() };
        let mut r = match book {
            Book::American => return Rules::for_level(Level::NFL),
            Book::Cfl => cfl(year, &mut code),
            Book::Arena => arena(year, &mut code),
            Book::NflEurope => {
                let mut r = nfl_era(year);
                // The two-point try all along (the NFL only took it in
                // 1994), and four points for a field goal from 50 yards.
                code.try_kick = Some(2.0);
                code.try_plays = [None, Some(2.0), None];
                code.long_field_goal = Some((50.0, 4));
                r.overtime = Overtime::SuddenDeath;
                r.quarter = 900.0;
                r
            }
            Book::Xfl2001 => {
                let mut r = nfl_era(year);
                // No kicked extra point: one point for a play from the 2,
                // and from the playoffs two from the 5 and three from the 10.
                code.try_kick = None;
                code.try_plays = [Some(2.0), Some(5.0), Some(10.0)];
                // No fair catch; the returner gets five yards, and a punt
                // that travels 25 yards is anybody's ball.
                code.fair_catch = false;
                code.halo = 5.0;
                code.punt_live = true;
                // One back may run toward the line at the snap.
                code.waggle = Waggle::One;
                r.overtime = Overtime::Xfl2001;
                r.play_clock = 35.0;
                r
            }
            Book::Xfl => {
                let mut r = nfl_era(year);
                set_up_kickoff(&mut r, &mut code, 30.0, 35.0, 45.0, 35.0);
                code.try_kick = None;
                code.try_plays = [Some(2.0), Some(5.0), Some(10.0)];
                code.punt_touchback_35 = true;
                if year >= 2023 {
                    // 2023: fourth and 15 from the own 25 instead of an
                    // onside kick.
                    code.onside_alt = Some(OnsideAlt { from: 25.0, togo: 15.0, behind: 1, minutes: 60.0 });
                }
                r.overtime = Overtime::Shootout { rounds: if year >= 2023 { 3 } else { 5 }, from: 5 };
                r.timeouts = 2;
                r.play_clock = 25.0 + 7.0;
                r
            }
            Book::Aaf => {
                let mut r = nfl_era(year);
                // No kickoffs: the ball at the 25 after a score and to
                // start each half; no kicked extra point, a two-point play
                // from the 2; fourth and 12 from the 28 when trailing by 17
                // or in the last five minutes; at most five rushers.
                code.kickoffs = false;
                code.no_kick_start = 25.0;
                code.try_kick = None;
                code.try_plays = [None, Some(2.0), None];
                code.onside_alt = Some(OnsideAlt { from: 28.0, togo: 12.0, behind: 17, minutes: 5.0 });
                code.max_rushers = 5;
                r.overtime = Overtime::Aaf;
                r.play_clock = 35.0;
                r
            }
            Book::Usfl1983 => {
                let mut r = nfl_era(year);
                // NFL rules with the two-point try (from the 2).
                code.try_kick = Some(2.0);
                code.try_plays = [None, Some(2.0), None];
                r.overtime = Overtime::SuddenDeath;
                r
            }
            Book::Usfl2022 => {
                let mut r = nfl_era(year);
                // A kicked extra point, two from the 2 or three from the 10;
                // fourth and 12 from the own 33 instead of an onside kick;
                // a best-of-three shootout from the 2 in overtime.
                r.kickoff_from = 25.0;
                r.kickoff_touchback = 25.0;
                code.try_kick = Some(15.0);
                code.try_plays = [None, Some(2.0), Some(10.0)];
                code.onside_alt = Some(OnsideAlt { from: 33.0, togo: 12.0, behind: 1, minutes: 60.0 });
                r.overtime = Overtime::Shootout { rounds: 3, from: 2 };
                r
            }
            Book::Ufl2009 => {
                let r = nfl_era(year);
                code.try_kick = Some(r.xp_from);
                code.try_plays = [None, Some(2.0), None];
                r
            }
            Book::Ufl => {
                let mut r = nfl_era(year);
                set_up_kickoff(&mut r, &mut code, 30.0, 35.0, 45.0, 35.0);
                code.punt_touchback_35 = true;
                code.onside_alt = Some(OnsideAlt { from: 28.0, togo: 12.0, behind: 1, minutes: 60.0 });
                r.overtime = Overtime::Shootout { rounds: 3, from: 5 };
                r.play_clock = 25.0 + 7.0;
                if year >= 2026 {
                    // 2026: a kicked point from the 15 again, two from the
                    // 2, three from the 8; four points for a field goal from
                    // 60 yards; no punting from the opponent's side of the
                    // 50 outside the last two minutes of a half; one foot
                    // in bounds.
                    code.try_kick = Some(15.0);
                    code.try_plays = [None, Some(2.0), Some(8.0)];
                    code.long_field_goal = Some((60.0, 4));
                    code.no_punt_past_50 = true;
                    r.feet_inbounds = 1;
                } else {
                    code.try_kick = None;
                    code.try_plays = [Some(2.0), Some(5.0), Some(10.0)];
                }
                r
            }
        };
        if code.try_kick.is_none() && code.try_plays.iter().all(|p| p.is_none()) {
            code.try_kick = Some(r.xp_from);
        }
        if let Some(k) = code.try_kick {
            r.xp_from = k;
        }
        if let Some(t) = code.try_plays[1] {
            r.two_point_from = t;
        }
        code.field.downs = code.field.downs.max(3);
        r.code = code;
        r
    }

    /// Downs to make ten yards (four; three in the CFL).
    pub fn downs(&self) -> u8 {
        self.code.field.downs
    }
}

/// The XFL's kickoff (2020 on, taken by the UFL and, in 2025, by the NFL):
/// the kicker at his 30, the cover team at the receiving 35, the return
/// team at its 30; nobody moves until the ball is caught or lands. A kick
/// short of the 20 or out of bounds comes out to the 45, one into the end
/// zone to the 35.
fn set_up_kickoff(r: &mut Rules, code: &mut Code, from: f64, touchback: f64, short: f64, cover: f64) {
    r.nfl_kickoff = true;
    r.kickoff_from = from;
    r.kickoff_touchback = touchback;
    code.short_kick_spot = short;
    code.kick_out_spot = short;
    code.cover_line = cover;
}

/// The CFL by season.
fn cfl(year: i32, code: &mut Code) -> Rules {
    let mut r = Rules::for_level(Level::NFL);
    // 12 a side on a field 110 yards between the goal lines and 65 wide;
    // 20-yard end zones from 1986 (25 before); from 2027 100 yards and
    // 15-yard end zones with the posts at the back.
    code.players = 12;
    let (length, end_zone, post_depth) = match year {
        ..=1985 => (110.0, 25.0, 0.0),
        1986..=2026 => (110.0, 20.0, 0.0),
        _ => (100.0, 15.0, 15.0),
    };
    code.field = Field { length, width: 65.0, end_zone, downs: 3, posts: 3.083, post_depth, nets: false };
    // The defensive line a yard off the ball; every back may be moving
    // and receivers may run forward at the snap.
    code.line_off = 1.0;
    code.waggle = Waggle::All;
    // No fair catch: five yards for the returner.
    code.fair_catch = false;
    code.halo = 5.0;
    code.rouge = if year >= 2026 { Rouge::Settled } else { Rouge::Full };
    code.single_spot = 35.0;
    code.no_single_spot = 40.0;
    // The convert: a kick from the 5 and a two-point try from the 5; from
    // 2015 the kick from the 25 (a 32-yard try) and two from the 3.
    if year >= 2015 {
        code.try_kick = Some(25.0);
        code.try_plays = [None, Some(3.0), None];
    } else {
        code.try_kick = Some(5.0);
        code.try_plays = [None, Some(5.0), None];
    }
    code.warning = 180.0;
    code.late_every_play = true;
    code.timeouts_per_game = year >= 2013;
    code.challenges = match year {
        ..=2006 => 0,
        2007..=2013 => 1,
        _ => 2,
    };
    r.kickoff_from = 35.0;
    r.kickoff_touchback = 35.0;
    r.nfl_kickoff = false;
    // Hash marks 24 yards in from each sideline.
    r.hash = 65.0 / 2.0 - 24.0;
    r.feet_inbounds = 1;
    r.two_minute_warning = true;
    r.first_down_stop = FirstDownStop::Never;
    // Overtime: possessions from the other team's 35 (from 2000), with a
    // two-point convert required after a touchdown from 2010.
    r.overtime = Overtime::Cfl { two_from: 2010 };
    r.timeouts = if year >= 2013 { 2 } else { 1 };
    // 20 seconds from the referee's signal (about 27 from the whistle); a
    // 35-second clock from the end of the play from 2026.
    r.play_clock = if year >= 2026 { 35.0 } else { 27.0 };
    r.down_on_fall = true;
    r
}

/// Arena football by season.
fn arena(year: i32, code: &mut Code) -> Rules {
    let mut r = Rules::for_level(Level::NFL);
    // Eight a side on a padded field 50 yards long and 85 feet wide, with
    // 8-yard end zones, goalposts 9 feet wide at the end line and rebound
    // nets beside them.
    code.players = 8;
    code.field = Field {
        length: 50.0,
        width: 85.0 / 3.0,
        end_zone: 8.0,
        downs: 4,
        posts: 1.5,
        post_depth: 8.0,
        nets: true,
    };
    code.waggle = Waggle::One;
    code.punting = false;
    code.drop_kick = true;
    code.try_kick = Some(3.0);
    code.try_plays = [None, Some(3.0), None];
    code.arena_kickoff = true;
    code.max_rushers = 4;
    code.man_only = true;
    code.warning = 0.0;
    code.running_clock = 60.0;
    code.ironman = year <= 2006;
    // Kickoffs from the goal line; a kick downed in the end zone or dead
    // beyond it comes out to the 5.
    r.kickoff_from = 0.0;
    r.kickoff_touchback = 5.0;
    r.nfl_kickoff = false;
    r.touchback = 5.0;
    r.hash = 1.5;
    r.feet_inbounds = 1;
    r.two_minute_warning = false;
    r.overtime = Overtime::Arena;
    r.play_clock = 35.0;
    r.down_on_fall = false;
    r
}

/// The width of the American field (for the books' docs and checks).
pub const AMERICAN_WIDTH: f64 = WIDTH;
