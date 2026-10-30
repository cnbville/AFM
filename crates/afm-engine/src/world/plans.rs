//! The deeper game plan for your team (update 2, lane 5: G1-G23). The
//! Plans page holds the program's decisions: your identity (the head
//! coach's guidelines, `world::guide`), the standing game rules and the
//! call words, the self-scout, the call sheet your coordinators draft each
//! week, and the installs. Game day's Game plan and Report tabs hold the
//! week's plan against one opponent: the answer to each of their personnel
//! groups, snap limits, special teams, instructions and checks, the "if
//! they do this, we go to that" lines and the keys to the game.
//!
//! Everything here reaches the engine through your game team's prep
//! (`prepare`), the same way as your practice week. Sizes are Claude's
//! picks (proposed, not confirmed).

use super::World;
use crate::calendar::{Date, Level};
use crate::game::calling::{Answer, DefCtx, Memory, OffCtx, Situation};
use crate::game::guide::{self, Guidelines};
use crate::game::playbook::{
    Backfield, Coverage, DefCall, Family, Formation, OffCall, PASS_CONCEPTS, PassDepth, Personnel, PlayKind, Pressure,
    call_family, pass_concept,
};
use crate::game::roster::{DefSheetCall, SheetCall, Teams};
use crate::game::sheet;
use crate::game::{Film, GameTeam, PlayType};
use crate::rng::{Rng, clamp};
use crate::tendencies::Instruction;
use crate::training::seat::{SheetKind, SheetPlay};

/// Your plans, kept with the career.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SeatPlans {
    /// Your high school's systems as this season's summer camp installed
    /// them (G2, S5): the season and the install.
    pub hs_install: Option<(i32, super::systems::Installed)>,
    /// Your identity: the head coach's guidelines (G1).
    pub guide: Guidelines,
    /// When you last changed it.
    pub guide_set: Option<Date>,
    /// When you last talked the identity through with each coordinator
    /// (coach index, date): he takes it better for a month.
    pub talks: Vec<(usize, Date)>,
    /// The call words (G5).
    pub words: Words,
    /// This week's call sheet as your coordinators drafted it (G7-G13).
    pub draft: Option<Draft>,
    /// The week's plan against one opponent (G16-G21).
    pub week: Week,
    /// The keys to each of the last games, checked after it (G21).
    pub keys: Vec<KeysCheck>,
    /// Package depth (G15): per board, the second and third man for each
    /// slot, in slot order (0 none).
    pub depth: Vec<(crate::training::seat::Board, Vec<u32>)>,
    /// The defensive line rotation by series (G15): id and role (1 whole
    /// series, 2 every other series, 3 rush downs only).
    pub rotation: Vec<(u32, u8)>,
    /// The last month the coordinators' trust moved for the identity.
    pub trust_month: Option<(i32, u8)>,
    /// The team's grasp of the week's plan each day (G14): offense,
    /// defense and the kicking game, 0-1 (the last two weeks).
    pub grasp_log: Vec<(Date, [f32; 3])>,
    /// The day of the last full self-scout (a bye week's study, G6): the
    /// tells read from fewer snaps for the rest of the season.
    pub study: Option<Date>,
}

/// The call words (G5): the season each side's words were set, and the
/// day you changed them last (the learning that week).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Words {
    pub since: [i32; 2],
    pub changed: [Option<Date>; 2],
    /// The season the words were set before the last change (to undo it).
    pub prev: [i32; 2],
}

/// A play on the drafted sheet.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Play {
    Off(SheetPlay),
    Def(Coverage, Pressure),
}

/// One line on the drafted sheet: its box (with the hash, `sheet::key`),
/// the play, the coordinator's reason, what a script play tests, and its
/// marks.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Entry {
    pub k: u8,
    pub play: Play,
    pub why: String,
    #[serde(default)]
    pub tests: String,
    /// Bits: 1 the quarterback's favourite, 2 a tendency breaker, 4 repped
    /// badly (the coordinator proposes a cut), 8 yours (you added it), 16
    /// the bad-weather sheet.
    pub marks: u8,
}

pub const FAV: u8 = 1;
pub const BREAKER: u8 = 2;
pub const CUT: u8 = 4;
pub const YOURS: u8 = 8;
pub const WET: u8 = 16;

/// A box holds the plays his play caller calls there: enough for this
/// share of his calls, the box's own size at least and a card's room at
/// most (an offense box a hash, a defense box; proposed, not confirmed).
pub const COVER: f64 = 0.98;
pub const BOX_MAX: usize = 30;
pub const DBOX_MAX: usize = 12;

/// How many of a box's plays, most called first, hold `share` of his calls.
fn cover<T>(counts: &[(T, f64)], share: f64) -> usize {
    let total: f64 = counts.iter().map(|x| x.1).sum();
    let mut acc = 0.0;
    for (i, x) in counts.iter().enumerate() {
        acc += x.1;
        if acc >= share * total - 1e-9 {
            return i + 1;
        }
    }
    counts.len()
}

/// The sheet's six parts, for your review.
pub const PARTS: [&str; 6] = [
    "Opening script",
    "Base downs, 1st and 2nd",
    "3rd downs",
    "Red zone and goal line",
    "Backed up, the clock and two-minute",
    "Shots and two-point",
];

pub fn part_of(k: u8) -> usize {
    match sheet::split(k).0 {
        sheet::SCRIPT => 0,
        sheet::FIRST | sheet::SECOND_LONG | sheet::SECOND_MEDIUM | sheet::SECOND_SHORT => 1,
        sheet::THIRD_SHORT | sheet::THIRD_MEDIUM | sheet::THIRD_LONG => 2,
        sheet::RED_ZONE | sheet::GOAL_LINE => 3,
        sheet::BACKED_UP | sheet::FOUR_MINUTE | sheet::TWO_MINUTE => 4,
        _ => 5,
    }
}

/// The notes you can send a draft back with.
pub const NOTES: [&str; 9] = [
    "More runs",
    "More throws",
    "More of the quarterback's favourites",
    "Break our tendencies",
    "Fewer new plays",
    "More man",
    "More zone",
    "More pressure",
    "Less pressure",
];

/// The week's call sheet as drafted, with your review.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Draft {
    pub game: Date,
    pub opp: usize,
    pub drafted: Date,
    /// The coordinators who drafted each side.
    pub by: [Option<usize>; 2],
    pub off: Vec<Entry>,
    pub def: Vec<Entry>,
    /// Each part's state per side: 0 waiting for you, 1 approved, 2 sent
    /// back and redrafted.
    pub status: [[u8; 6]; 2],
    /// Your edits per part and side.
    pub edits: [[u8; 6]; 2],
    /// Your notes, by `NOTES`, per side, and the redraft round.
    pub notes: [Vec<u8>; 2],
    pub round: u8,
}

/// The week's plan against one opponent (G16-G21). None and empty fields
/// leave the staff's own.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Week {
    pub game: Option<Date>,
    pub answers: [Answer; 4],
    pub limits: Vec<(u32, u16)>,
    pub teams: Teams,
    /// Instructions for this opponent: player id and the instruction's key
    /// (`ORDERS`).
    pub orders: Vec<(u32, u8)>,
    pub checks: Option<u8>,
    pub ifs: Option<Vec<u8>>,
    pub base_only: Vec<u32>,
    /// Extra film tonight (1) and an extra walkthrough (2), per player.
    pub extra: Vec<(u32, u8)>,
    pub keys: Vec<Key>,
    /// You locked the week's plan: the staff's own checks and "if they do
    /// this" lines are written in as they stand.
    pub locked: bool,
}

/// A key to the game (G21): its kind and the bar.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Key {
    pub kind: u8,
    pub bar: f32,
    pub text: String,
}

/// A game's keys checked after it.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct KeysCheck {
    pub date: Date,
    pub opp: String,
    /// The key, whether we met it, what happened.
    pub keys: Vec<(String, bool, String)>,
}

/// Instructions you can give for one opponent (G19).
pub const ORDERS: [(Instruction, &str); 7] = [
    (Instruction::Slide, "QB"),
    (Instruction::ProtectBall, "QB"),
    (Instruction::TakeShots, "QB"),
    (Instruction::HighAndTight, "ball carriers"),
    (Instruction::KeepContain, "pass rushers"),
    (Instruction::StayHome, "coverage"),
    (Instruction::WrapUp, "tacklers"),
];

/// Your next game: its date, the opponent, home or away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Next {
    pub date: Date,
    pub opp: usize,
    pub home: bool,
    pub level: Level,
}

/// Your next game from today (today's included).
pub fn next_game(w: &World) -> Option<Next> {
    upcoming(w).into_iter().next()
}

/// Your games still to play this season from today (today's included),
/// in order: the schedule, the playoff games once they are set (an NFL
/// round and the Super Bowl, a high school bracket's round, the other
/// leagues' playoffs) and a game being played today.
pub fn upcoming(w: &World) -> Vec<Next> {
    let Some((level, t)) = w.seat_team() else { return Vec::new() };
    let today = w.today;
    let mut v: Vec<(Date, usize, usize)> = match level {
        Level::College => w
            .college_fixtures(Some(t))
            .into_iter()
            .filter(|f| f.score.is_none() && !f.cancelled)
            .map(|f| (f.date, f.home, f.away))
            .collect(),
        Level::Nfl => {
            let mut v: Vec<(Date, usize, usize)> = w
                .nfl_fixtures(Some(t))
                .into_iter()
                .filter(|f| !f.played)
                .filter_map(|f| Some((f.when?, f.home, f.away)))
                .collect();
            if let Some(s) = w.nfl_season.as_ref() {
                // A playoff round's games are dropped once played.
                v.extend(s.round_games.iter().filter(|g| g.2 == t || g.3 == t).map(|g| (g.4.date, g.2, g.3)));
                if s.booked
                    && s.champion.is_none()
                    && let (Some(&a), Some(&b)) = (s.alive[0].first(), s.alive[1].first())
                    && (a == t || b == t)
                {
                    v.push((w.dates.super_bowl, a, b));
                }
            }
            v
        }
        Level::HighSchool => {
            let mut v: Vec<(Date, usize, usize)> =
                w.hs_fixtures(t).into_iter().map(|f| (f.date, f.home, f.away)).collect();
            v.extend(w.hs_playoff_next(t));
            v
        }
        Level::Other => super::pro::other::seat_fixtures(w),
        _ => Vec::new(),
    };
    // A game being played today, as the day built it.
    if let Some(l) = w.live.as_ref().filter(|l| l.date == today && l.teams.contains(&t))
        && !v.iter().any(|g| g.0 == today)
    {
        v.push((today, l.teams[0], l.teams[1]));
    }
    let mut out: Vec<Next> = v
        .into_iter()
        .filter(|g| g.0 >= today)
        .map(|(date, h, a)| Next { date, opp: if h == t { a } else { h }, home: h == t, level })
        .collect();
    out.sort_by_key(|n| n.date);
    out.dedup_by_key(|n| n.date);
    out
}

/// A team's name at a level.
pub fn team_name(w: &World, level: Level, i: usize) -> String {
    match level {
        Level::Nfl => w.nfl.get(i).map_or(String::new(), |t| t.name.to_string()),
        Level::HighSchool => w.school_name(i).to_string(),
        Level::Other => super::pro::other::name(i, w.today.year()).to_string(),
        _ => w.program_name(i).to_string(),
    }
}

/// The day the coordinators draft the sheet for a game: Tuesday for a
/// Sunday NFL game (five days out), Sunday for a Saturday (six), the
/// weekend for a high school's Friday (six); never before the last game.
pub fn plan_day(level: Level, game: Date) -> Date {
    game.add(if pro(level) { -5 } else { -6 })
}

/// A pro league's level: the NFL, or another league's (its book's rules
/// on an NFL roster's football).
pub fn pro(level: Level) -> bool {
    matches!(level, Level::Nfl | Level::Other)
}

/// How much more your league's play callers throw than the NFL's habits
/// say (another league's book; 0 elsewhere).
pub fn book_lean(w: &World) -> f64 {
    match w.seat.filter(|s| s.level == Level::Other).and_then(|s| s.other) {
        Some(t) => {
            let book = super::pro::other::book(super::pro::other::league_of(t));
            crate::game::Rules::for_book(book, w.today.year()).code.pass_lean
        }
        None => 0.0,
    }
}

/// Your team as the engine would build it now (for drafting and reads).
pub fn our_team(w: &World) -> Option<GameTeam> {
    let (level, t) = w.seat_team()?;
    match level {
        // The season's team, else the team as the engine would dress it
        // today (the offseason).
        Level::College => w
            .college
            .as_ref()
            .and_then(|c| c.teams.get(t))
            .and_then(|x| x.clone())
            .map(|mut x| {
                crate::training::seat::sit_hurt(w, &mut x);
                x
            })
            .or_else(|| w.seat_game_team().map(|x| x.0)),
        Level::Nfl => super::pro::games::game_side(w, t).map(|s| s.team),
        Level::HighSchool => super::hs_year::seat_team(w, t),
        _ => w.seat_game_team().map(|x| x.0),
    }
}

/// The opponent's film: the tapes your staff keeps of their games, else
/// their season's film.
pub fn opp_film(w: &World, level: Level, opp: usize) -> Film {
    let mut f = Film::default();
    let mut n = 0;
    for t in w.gameday.tapes.iter().filter(|t| t.teams.contains(&opp)) {
        let side = if t.teams[0] == opp { 0 } else { 1 };
        f.add(&t.film[side]);
        n += 1;
    }
    if n == 0 {
        match level {
            Level::College => f = w.programs.get(opp).map(|p| p.film.clone()).unwrap_or_default(),
            Level::Nfl => f = w.clubs.get(opp).map(|c| c.film.clone()).unwrap_or_default(),
            _ => {}
        }
    }
    f
}

fn key_of(w: &World, game: Date, opp: usize, round: u8) -> u32 {
    w.options.seed ^ (game.0 as u32).wrapping_mul(2_654_435_761) ^ (opp as u32).rotate_left(13) ^ round as u32
}

/// A neutral situation for a box (score level, mid-game).
fn sit(down: u8, togo: f64, yl: f64, college: bool) -> Situation {
    Situation {
        down,
        togo,
        yardline_100: yl,
        score_diff: 0,
        quarter: 2,
        half_left: 600.0,
        game_left: 2400.0,
        timeouts: 3,
        opp_timeouts: 3,
        ease: 0.0,
        concede: false,
        def_ease: 0.0,
        def_concede: false,
        college,
        lean: 0.0,
    }
}

/// The situations a box is drafted from, and how many plays it gets.
fn box_spec(k: u8, college: bool, two_from: f64) -> (Vec<Situation>, usize) {
    let s = |d: u8, t: f64, y: f64| sit(d, t, y, college);
    match k {
        sheet::FIRST => (vec![s(1, 10.0, 75.0), s(1, 10.0, 60.0), s(1, 10.0, 45.0), s(1, 10.0, 35.0)], 8),
        sheet::SECOND_LONG => (vec![s(2, 8.0, 70.0), s(2, 10.0, 55.0), s(2, 9.0, 40.0)], 6),
        sheet::SECOND_MEDIUM => (vec![s(2, 5.0, 65.0), s(2, 4.0, 45.0)], 5),
        sheet::SECOND_SHORT => (vec![s(2, 2.0, 60.0), s(2, 1.0, 45.0)], 4),
        sheet::THIRD_LONG => (vec![s(3, 8.0, 65.0), s(3, 11.0, 50.0)], 6),
        sheet::THIRD_MEDIUM => (vec![s(3, 5.0, 60.0), s(3, 4.0, 45.0)], 5),
        sheet::THIRD_SHORT => (vec![s(3, 2.0, 55.0), s(3, 1.0, 45.0)], 4),
        sheet::RED_ZONE => (vec![s(1, 10.0, 15.0), s(2, 6.0, 12.0), s(3, 4.0, 9.0), s(1, 10.0, 19.0)], 6),
        sheet::GOAL_LINE => (vec![s(1, 2.0, 2.0), s(2, 1.0, 1.0), s(3, 2.0, 2.0)], 4),
        sheet::BACKED_UP => (vec![s(1, 10.0, 95.0), s(2, 7.0, 93.0)], 3),
        sheet::FOUR_MINUTE => {
            let mut a = s(1, 10.0, 60.0);
            a.quarter = 4;
            a.score_diff = 6;
            a.game_left = 250.0;
            a.half_left = 250.0;
            let mut b = a;
            b.down = 2;
            b.togo = 6.0;
            (vec![a, b], 4)
        }
        sheet::TWO_MINUTE => {
            let mut a = s(1, 10.0, 70.0);
            a.quarter = 4;
            a.score_diff = -3;
            a.half_left = 100.0;
            a.game_left = 100.0;
            let mut b = a;
            b.down = 2;
            b.togo = 7.0;
            b.yardline_100 = 45.0;
            (vec![a, b], 6)
        }
        sheet::SHOTS => (vec![s(2, 2.0, 50.0), s(1, 10.0, 45.0)], 4),
        sheet::TWO_POINT => (vec![s(1, two_from, two_from)], 3),
        _ => (vec![s(1, 10.0, 75.0), s(2, 8.0, 70.0), s(1, 10.0, 55.0)], 15),
    }
}

/// The boxes each side drafts, in order.
pub const OFF_BOXES: [u8; 15] = [
    sheet::SCRIPT,
    sheet::FIRST,
    sheet::SECOND_LONG,
    sheet::SECOND_MEDIUM,
    sheet::SECOND_SHORT,
    sheet::THIRD_LONG,
    sheet::THIRD_MEDIUM,
    sheet::THIRD_SHORT,
    sheet::RED_ZONE,
    sheet::GOAL_LINE,
    sheet::BACKED_UP,
    sheet::FOUR_MINUTE,
    sheet::TWO_MINUTE,
    sheet::SHOTS,
    sheet::TWO_POINT,
];

/// The boxes split by hash: the downs between the 20s.
fn by_hash(k: u8) -> bool {
    matches!(
        k,
        sheet::FIRST
            | sheet::SECOND_LONG
            | sheet::SECOND_MEDIUM
            | sheet::SECOND_SHORT
            | sheet::THIRD_LONG
            | sheet::THIRD_MEDIUM
            | sheet::THIRD_SHORT
    )
}

fn to_sheet(c: &OffCall, hash: u8) -> Option<SheetPlay> {
    let play = match c.kind {
        PlayKind::Run(r) | PlayKind::Rpo(r, _) => SheetKind::Run(r),
        PlayKind::Pass(i) => SheetKind::Pass(i as u16),
        _ => return None,
    };
    Some(SheetPlay {
        personnel: c.formation.personnel,
        backfield: c.formation.backfield,
        set: c.formation.set,
        play,
        hash,
    })
}

/// A sheet play's name, as the coordinator says it.
pub fn play_name(p: &SheetPlay, designs: &[crate::training::seat::SavedDesign]) -> String {
    let what = match p.play {
        SheetKind::Run(r) => r.name().to_string(),
        SheetKind::Pass(i) => pass_concept(i as usize).name.to_string(),
        SheetKind::Design(k) => designs.get(k as usize).map_or("your design".to_string(), |d| d.design.name.clone()),
    };
    let back = match p.backfield {
        Backfield::Shotgun => "gun",
        Backfield::Pistol => "pistol",
        Backfield::UnderCenter => "under center",
    };
    format!("{} {} {} · {}", p.personnel.name(), back, p.set.name(), what)
}

fn is_run(p: &SheetPlay) -> bool {
    matches!(p.play, SheetKind::Run(_))
}

/// A box's plays (sorted by weight): runs and throws in the share the play
/// caller calls them there, each kind best first, spread through the list
/// so the order of preference keeps the mix. The most frequent plays alone
/// would fill a box with runs: his runs gather on a few plays while his
/// throws spread over many concepts, and a box with no throw turns his
/// throw into a run on game day (`game::sheet::off_call`).
fn mix(counts: Vec<(SheetPlay, f64)>, want: usize) -> Vec<SheetPlay> {
    let (runs, passes): (Vec<_>, Vec<_>) = counts.into_iter().partition(|x| is_run(&x.0));
    let wr: f64 = runs.iter().map(|x| x.1).sum();
    let wp: f64 = passes.iter().map(|x| x.1).sum();
    let mut nr = if wr + wp > 0.0 { (want as f64 * wr / (wr + wp)).round() as usize } else { 0 };
    if want >= 2 && !runs.is_empty() && !passes.is_empty() {
        nr = nr.clamp(1, want - 1);
    }
    let mut nr = nr.min(runs.len());
    let np = (want - nr).min(passes.len());
    if nr + np < want {
        nr = (want - np).min(runs.len());
    }
    let mut keyed: Vec<(f64, SheetPlay)> = Vec::new();
    for (i, x) in runs.into_iter().take(nr).enumerate() {
        keyed.push(((i as f64 + 0.5) / nr as f64, x.0));
    }
    for (i, x) in passes.into_iter().take(np).enumerate() {
        keyed.push(((i as f64 + 0.5) / np as f64, x.0));
    }
    keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
    keyed.into_iter().map(|x| x.1).collect()
}

fn family_of(p: &SheetPlay) -> Family {
    match p.play {
        SheetKind::Run(r) => r.family(),
        SheetKind::Pass(i) => call_family(PlayKind::Pass(i as usize)),
        SheetKind::Design(_) => Family::Dropback,
    }
}

/// The football season a day belongs to (January's bowls and playoffs
/// close the last one).
pub fn season_of(d: Date) -> i32 {
    if d.month() <= 6 { d.year() - 1 } else { d.year() }
}

/// The season the self-scout reads: this one once you have played in it,
/// else the last one (the tape a staff studies in the offseason and in
/// camp).
pub fn board_season(w: &World) -> i32 {
    let now = season_of(w.today);
    let Some((level, t)) = w.seat_team() else { return now };
    let played = w.seat_games().iter().any(|g| g.level == level && season_of(g.date) == now && g.teams.contains(&t));
    if played { now } else { now - 1 }
}

/// Your kept games of that season, at this level and with this team (a
/// game from another job is not your team's film), oldest first, with
/// your side.
pub fn own_games(w: &World) -> Vec<(&super::seat::SeatGame, usize)> {
    let Some((level, t)) = w.seat_team() else { return Vec::new() };
    let season = board_season(w);
    w.seat_games()
        .iter()
        .filter(|g| g.level == level && season_of(g.date) == season && g.teams.contains(&t))
        .map(|g| (g, if g.teams[0] == t { 0 } else { 1 }))
        .collect()
}

/// A snap the self-scout counts: a run or a throw on a real down, outside
/// the two-minute drills and garbage time (both play to the clock and the
/// score, not to habit).
pub fn scout_counts(r: &crate::game::stats::PlayRecord) -> bool {
    !r.nullified
        && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
        && !(matches!(r.quarter, 2 | 4) && r.clock <= 120.0)
        && r.garbage == [0, 0]
        && r.down >= 1
}

/// Your starting quarterback as the team dresses (the depth order's first).
pub fn starting_qb(team: &GameTeam, level: Level) -> Option<u32> {
    let margin = if level == Level::HighSchool { 4.0 } else { 99.0 };
    let d = crate::game::roster::depth_chart(team, margin);
    d.list(crate::positions::Position::QB).first().map(|&i| team.players[i].id)
}

/// What your quarterback likes to throw (G12): the concepts he himself has
/// completed and gained on most in your games this season, eight throws
/// at least (a favourite is earned only through his own reps).
pub fn favourites(w: &World, qb: Option<u32>) -> Vec<(usize, u32, f64)> {
    let Some(qb) = qb else { return Vec::new() };
    let mut by: Vec<(usize, u32, f64)> = Vec::new();
    for (g, side) in own_games(w) {
        for r in g
            .stats
            .plays
            .iter()
            .filter(|r| r.offense == side && r.kind == PlayType::Pass && !r.nullified && r.passer == Some(qb))
        {
            let Some(c) = concept_in(&r.call) else { continue };
            match by.iter_mut().find(|x| x.0 == c) {
                Some(x) => {
                    x.1 += 1;
                    x.2 += r.epa;
                }
                None => by.push((c, 1, r.epa)),
            }
        }
    }
    by.retain(|x| x.1 >= 8);
    by.sort_by(|a, b| (b.2 / b.1 as f64).total_cmp(&(a.2 / a.1 as f64)));
    by.truncate(3);
    by
}

/// The pass concept named in a call's words (the longest name found).
pub fn concept_in(call: &str) -> Option<usize> {
    PASS_CONCEPTS
        .iter()
        .enumerate()
        .filter(|(_, c)| call.contains(c.name))
        .max_by_key(|(_, c)| c.name.len())
        .map(|(i, _)| i)
}

/// A look the film gives away (G6): the look, its runs and throws in your
/// games this season, and in how many games it showed.
#[derive(Clone, Debug, PartialEq)]
pub struct Tell {
    pub look: usize,
    pub runs: u32,
    pub passes: u32,
    pub games: u32,
}

impl Tell {
    pub fn snaps(&self) -> u32 {
        self.runs + self.passes
    }
    pub fn run_share(&self) -> f64 {
        self.runs as f64 / self.snaps().max(1) as f64
    }
}

/// Your offense's looks this season with their run and pass counts (from
/// your kept games' play logs, the snaps the self-scout counts).
pub fn own_looks(w: &World) -> Vec<Tell> {
    let mut out: Vec<Tell> = Vec::new();
    for (g, side) in own_games(w) {
        let mut seen: Vec<usize> = Vec::new();
        for r in g.stats.plays.iter().filter(|r| r.offense == side && scout_counts(r)) {
            let run = match r.kind {
                PlayType::Run => true,
                PlayType::Pass | PlayType::Sack | PlayType::Scramble => false,
                _ => continue,
            };
            let Some(look) = look_in(&r.call, &r.personnel) else { continue };
            let e = match out.iter_mut().position(|x| x.look == look) {
                Some(i) => &mut out[i],
                None => {
                    out.push(Tell { look, runs: 0, passes: 0, games: 0 });
                    out.last_mut().unwrap()
                }
            };
            if run {
                e.runs += 1;
            } else {
                e.passes += 1;
            }
            if !seen.contains(&look) {
                seen.push(look);
                e.games += 1;
            }
        }
    }
    out.sort_by_key(|x| std::cmp::Reverse(x.snaps()));
    out
}

/// The look (personnel, backfield, set) in a call's words.
pub fn look_in(call: &str, personnel: &str) -> Option<usize> {
    let p = [Personnel::P11, Personnel::P12, Personnel::P21, Personnel::P10, Personnel::P22, Personnel::P13]
        .into_iter()
        .find(|p| call.starts_with(p.name()) || personnel == p.name())?;
    let b = if call.contains(" gun ") {
        Backfield::Shotgun
    } else if call.contains(" pistol ") {
        Backfield::Pistol
    } else {
        Backfield::UnderCenter
    };
    use crate::game::playbook::Set;
    let set = [Set::Doubles, Set::Trips, Set::Bunch, Set::Stack, Set::Empty, Set::Wing]
        .into_iter()
        .find(|s| call.contains(&format!(" {} ", s.name())))?;
    let mut f = Formation::plain(p, b, 1.0);
    f.set = set;
    Some(f.look())
}

/// A look's words: "12 under center wing".
pub fn look_words(look: usize) -> String {
    use crate::game::playbook::{SET_COUNT, Set};
    let pers = [Personnel::P11, Personnel::P12, Personnel::P21, Personnel::P10, Personnel::P22, Personnel::P13];
    let p = pers.get(look / (3 * SET_COUNT)).copied().unwrap_or(Personnel::P11);
    let b = ["under center", "gun", "pistol"][(look / SET_COUNT) % 3];
    let s = [Set::Doubles, Set::Trips, Set::Bunch, Set::Stack, Set::Empty, Set::Wing][look % SET_COUNT];
    format!("{} {} {}", p.name(), b, s.name())
}

/// The strong tells: looks with 8 snaps or more, seen in two games or more,
/// where one kind is 75% or more of the calls (a bye week's full study
/// reads them from 6 snaps and 70%). One game's habit isn't a tell a
/// defense trusts.
pub fn tells(w: &World, full: bool) -> Vec<Tell> {
    let full = full || studied(w);
    let (n, edge) = if full { (6, 0.7) } else { (8, 0.75) };
    own_looks(w)
        .into_iter()
        .filter(|t| t.snaps() >= n && t.games >= 2 && (t.run_share() >= edge || t.run_share() <= 1.0 - edge))
        .collect()
}

/// Whether your staff did a full self-scout this season (a bye week's
/// study, G6): it holds to the season's end.
pub fn studied(w: &World) -> bool {
    w.plans.study.is_some_and(|d| d.0 <= w.today.0 && season_of(d) == season_of(w.today))
}

/// A full self-scout: the staff spends an in-season bye week on its own
/// film (G6: "best on a bye"): a game played this season, the next one
/// more than eight days off.
pub fn study(w: &mut World) -> Result<(), String> {
    can_study(w)?;
    w.plans.study = Some(w.today);
    Ok(())
}

/// Whether today is a bye week your staff can spend on its own film, and
/// why not when it isn't.
pub fn can_study(w: &World) -> Result<(), String> {
    let Some(n) = next_game(w) else {
        return Err("Your staff studies its own film in a bye week, with the season under way.".into());
    };
    let now = season_of(w.today);
    let played = w.seat_team().is_some_and(|(level, t)| {
        w.seat_games().iter().any(|g| g.level == level && season_of(g.date) == now && g.teams.contains(&t))
    });
    if !played {
        return Err("Your staff studies its own film in a bye week: no game this season on tape yet.".into());
    }
    if n.date.0 - w.today.0 <= 8 {
        return Err(format!(
            "Your staff studies its own film in a bye week: the next game is in {} {} and the week belongs to the opponent.",
            n.date.0 - w.today.0,
            crate::world::folk::noun(n.date.0 - w.today.0, "day")
        ));
    }
    Ok(())
}

/// New call words for a side (G5): the people who left with the old ones
/// no longer read us, and the players learn the new words this week.
/// `undo` puts the old words back the same week.
pub fn change_words(w: &mut World, side: usize, undo: bool) -> Result<(), String> {
    let side = side.min(1);
    let year = w.today.year();
    let words = &mut w.plans.words;
    if undo {
        if words.changed[side].is_none() {
            return Err("Nothing to undo: the words haven't changed.".into());
        }
        words.since[side] = words.prev[side];
        words.changed[side] = None;
    } else {
        words.prev[side] = words.since[side];
        words.since[side] = year;
        words.changed[side] = Some(w.today);
    }
    w.refresh_seat_team();
    Ok(())
}

/// How well your players know a family: the mean of the starters' know.
fn family_know(team: &GameTeam, f: Family) -> f64 {
    let v: Vec<f64> = team.players.iter().take(30).map(|p| p.know[f as usize] as f64).collect();
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// The opponent's run share on a down from film, if seen.
fn film_pass(f: &Film, down: usize) -> Option<f64> {
    let d = f.by_down.get(down)?;
    (d[1] >= 6.0).then(|| d[0] as f64 / d[1] as f64)
}

fn top_coverage(f: &Film) -> Option<(Coverage, f64)> {
    let all: f32 = f.cov.iter().sum();
    if all < 10.0 {
        return None;
    }
    let (k, v) = f.cov.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1))?;
    Some((Coverage::ALL[k], *v as f64 / all as f64))
}

/// The coordinator's reason for an offensive play.
fn off_reason(p: &SheetPlay, k: u8, film: &Film, marks: u8, rng: &mut Rng) -> String {
    if marks & BREAKER != 0 {
        return "breaks our tendency from this look: they'll be sitting on the other thing".into();
    }
    if marks & FAV != 0 {
        return "the quarterback's favourite: he's hit it all year".into();
    }
    let blitz = if film.def_snaps > 0.0 { film.blitz as f64 / film.def_snaps.max(1.0) as f64 } else { 0.0 };
    let cov = top_coverage(film);
    let base = sheet::split(k).0;
    match p.play {
        SheetKind::Run(r) => {
            if base == sheet::GOAL_LINE || base == sheet::THIRD_SHORT || base == sheet::SECOND_SHORT {
                format!("{} gets us the yard behind our best blocks", r.name())
            } else if base == sheet::FOUR_MINUTE {
                "keeps the clock running and the ball safe".into()
            } else if let Some((c, s)) = cov.filter(|c| c.0.high() == 2) {
                format!("they play {} on {:.0}% of snaps: a light box to run at", c.name(), 100.0 * s)
            } else if blitz > 0.3 {
                format!("they bring five on {:.0}% of snaps: a run behind the blitz", 100.0 * blitz)
            } else {
                ["stays ahead of the chains", "our best run from this look", "sets up the play action off it"]
                    [rng.int(0, 2) as usize]
                    .into()
            }
        }
        SheetKind::Pass(i) => {
            let c = pass_concept(i as usize);
            if c.depth == PassDepth::Screen && blitz > 0.25 {
                format!("a screen behind a blitz they bring {:.0}% of the time", 100.0 * blitz)
            } else if c.play_action {
                "play action off our run look: their safeties come down".into()
            } else if base == sheet::SHOTS {
                "a shot when they're thinking run".into()
            } else if let Some((cv, s)) = cov {
                format!("beats the {} they played on {:.0}% of snaps", cv.name(), 100.0 * s)
            } else if base == sheet::THIRD_LONG || base == sheet::THIRD_MEDIUM {
                "routes past the sticks".into()
            } else {
                "the concept for what they've shown".into()
            }
        }
        SheetKind::Design(_) => "our own play, drawn for this".into(),
    }
}

/// What an opening script play tests (G11).
fn tests_of(p: &SheetPlay, i: usize) -> String {
    let t: &[&str] = if is_run(p) {
        &[
            "how they fit the run from this look",
            "whether a safety comes down to it",
            "their front against our base run",
            "who sets the edge",
        ]
    } else {
        &[
            "their coverage on early downs",
            "how they handle motion",
            "whether they bracket our best receiver",
            "their answer to play action",
            "their pressure on second down",
        ]
    };
    t[i % t.len()].to_string()
}

/// The defense's reason for a call.
fn def_reason(cov: Coverage, pr: Pressure, k: u8, film: &Film) -> String {
    let base = sheet::split(k).0;
    let pass = match base {
        sheet::FIRST => film_pass(film, 0),
        sheet::SECOND_LONG | sheet::SECOND_MEDIUM | sheet::SECOND_SHORT => film_pass(film, 1),
        sheet::THIRD_LONG | sheet::THIRD_MEDIUM | sheet::THIRD_SHORT => film_pass(film, 2),
        _ => None,
    };
    if pr.extra() > 0 {
        return match pass {
            Some(p) if p >= 0.6 => format!("they throw {:.0}% here: make him get it out", 100.0 * p),
            _ => "pressure to speed him up".into(),
        };
    }
    match pass {
        Some(p) if p >= 0.65 && cov.high() == 2 => format!("they throw {:.0}% here: keep it in front", 100.0 * p),
        Some(p) if p <= 0.45 && cov.high() < 2 => format!("they run {:.0}% here: a safety down", 100.0 * (1.0 - p)),
        _ if base == sheet::GOAL_LINE => "everybody in the box".into(),
        _ if cov.is_man() => "man on their receivers: our corners can hold up".into(),
        _ => format!("{} for what they showed on tape", cov.name()),
    }
}

fn biases(notes: &[u8]) -> (f64, f64, bool, bool, f64, f64) {
    // (runs factor, passes factor, favourites first, breakers, man shift,
    // pressure shift)
    let mut v = (1.0, 1.0, false, false, 0.0, 0.0);
    for &n in notes {
        match n {
            0 => v.0 *= 1.8,
            1 => v.1 *= 1.8,
            2 => v.2 = true,
            3 => v.3 = true,
            5 => v.4 += 0.25,
            6 => v.4 -= 0.25,
            7 => v.5 += 0.15,
            8 => v.5 -= 0.15,
            _ => {}
        }
    }
    v
}

/// The coordinators' sheet for a game (G7-G13): each box from the calls
/// the play caller himself makes there, against what the opponent's film
/// shows, with the quarterback's favourites, breakers for the looks that
/// give us away and the plays repped badly marked for a cut. A redraft
/// takes your notes. Never empty: every box gets its plays.
pub fn draft_for(w: &World, next: &Next, notes: [&[u8]; 2], round: u8) -> Option<Draft> {
    let team = our_team(w)?;
    Some(draft_with(w, &team, next, notes, round))
}

/// The same draft from a game team in hand: building your team runs
/// `prepare`, which drafts, so `prepare` hands its own team in rather than
/// building it again.
pub fn draft_with(w: &World, team: &GameTeam, next: &Next, notes: [&[u8]; 2], round: u8) -> Draft {
    let qb = starting_qb(team, next.level);
    let t = &w.training;
    let designs = t
        .designs
        .iter()
        .enumerate()
        .filter(|(_, d)| d.installing)
        .take(2)
        .map(|(i, d)| {
            (i as u16, d.design.formation, t.install_level(crate::training::seat::InstallKey::Design(i as u16)))
        })
        .collect();
    // His quarterback's running, as the game reads it (the depth order's
    // first quarterback).
    let qb_run = qb.and_then(|id| team.players.iter().find(|p| p.id == id)).map_or(0.0, crate::game::flow::qb_run);
    let level = level_of(next.level);
    let inp = DraftIn {
        film: opp_film(w, next.level, next.opp),
        level,
        two_from: crate::game::Rules::for_level(level).two_point_from,
        qb_run,
        seed: key_of(w, next.date, next.opp, round),
        favs: favourites(w, qb).into_iter().map(|x| x.0).collect(),
        tells: tells(w, false),
        designs,
    };
    let (off, def) = draft_sides(team, &inp, notes);
    let (oc, dc) = callers(w);
    Draft {
        game: next.date,
        opp: next.opp,
        drafted: w.today,
        by: [oc, dc],
        off,
        def,
        status: [[0; 6]; 2],
        edits: [[0; 6]; 2],
        notes: [notes[0].to_vec(), notes[1].to_vec()],
        round,
    }
}

/// What the coordinators draft from: the opponent's film, the level of
/// play and where its two-point try is snapped, the starting
/// quarterback's running as the game reads it, the draft's own random key,
/// the quarterback's favourite concepts, the offense's tells and your
/// designs going in (playbook slot, formation, how far in).
pub struct DraftIn {
    pub film: Film,
    pub level: crate::player::Level,
    pub two_from: f64,
    pub qb_run: f64,
    pub seed: u32,
    pub favs: Vec<usize>,
    pub tells: Vec<Tell>,
    pub designs: Vec<(u16, Formation, f32)>,
}

/// Both sides of the coordinators' draft (G7-G13): the offense's boxes,
/// script, tendency breakers, designs and wet sheet; the defense's boxes,
/// script and wet sheet.
pub fn draft_sides(team: &GameTeam, inp: &DraftIn, notes: [&[u8]; 2]) -> (Vec<Entry>, Vec<Entry>) {
    let film = &inp.film;
    let college = inp.level != crate::player::Level::NFL;
    let mut rng = Rng::new(inp.seed);
    let mut mem = Memory::default();
    mem.study(film);
    let te = team.players.iter().filter(|p| p.position == crate::positions::Position::TE).count();
    let fb = team.players.iter().filter(|p| p.position == crate::positions::Position::RB).count() >= 2;
    let ctx = OffCtx {
        has_fb: fb,
        te_count: te.min(3),
        qb_run: inp.qb_run,
        level: inp.level,
        plan: team.plan().0,
        designs: [None; 8],
    };
    let staff = team.staff;
    let two_from = inp.two_from;
    let favs = &inp.favs;
    let tells = &inp.tells;
    let (runs_f, pass_f, fav_first, breakers, _, _) = biases(notes[0]);
    let mut off: Vec<Entry> = Vec::new();
    for &k in &OFF_BOXES {
        let (sits, n) = box_spec(k, college, two_from);
        for hash in if by_hash(k) { vec![1u8, 2] } else { vec![0u8] } {
            let want = if hash == 0 { n } else { n.div_ceil(2) };
            let mut counts: Vec<(SheetPlay, f64)> = Vec::new();
            // The script is his own first calls in its order (your notes
            // weigh which of them he keeps): his whole game in the share he
            // calls it, not his few most-called plays.
            let mut raw: Vec<SheetPlay> = Vec::new();
            let top = runs_f.max(pass_f) * if fav_first { 2.5 } else { 1.0 };
            for i in 0..140 {
                let s = &sits[i % sits.len()];
                let c = crate::game::calling::call_offense(&staff, &mem, s, &ctx, &mut rng);
                let Some(mut p) = to_sheet(&c, hash) else { continue };
                if k == sheet::SHOTS
                    && !matches!(p.play, SheetKind::Pass(i) if pass_concept(i as usize).depth == PassDepth::Deep || pass_concept(i as usize).play_action)
                {
                    continue;
                }
                p.hash = hash;
                let mut wt = if is_run(&p) { runs_f } else { pass_f };
                if fav_first && matches!(p.play, SheetKind::Pass(i) if favs.contains(&(i as usize))) {
                    wt *= 2.5;
                }
                if k == sheet::SCRIPT && raw.len() < want && rng.next() * top <= wt {
                    raw.push(p);
                }
                match counts.iter_mut().find(|x| x.0 == p) {
                    Some(x) => x.1 += wt,
                    None => counts.push((p, wt)),
                }
            }
            {
                // One line a play, in his most used formation for it, with
                // the weight of all of them: a box says which play.
                counts.sort_by(|a, b| b.1.total_cmp(&a.1));
                let mut plays: Vec<(SheetPlay, f64)> = Vec::new();
                for (p, w) in counts {
                    match plays.iter_mut().find(|x| x.0.play == p.play) {
                        Some(x) => x.1 += w,
                        None => plays.push((p, w)),
                    }
                }
                plays.sort_by(|a, b| b.1.total_cmp(&a.1));
                counts = plays;
            }
            // Never empty: a box with no fitting calls takes the base call.
            if counts.is_empty() {
                let f = Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0);
                let kind = if k == sheet::SHOTS { SheetKind::Pass(deep_concept() as u16) } else { SheetKind::Pass(0) };
                counts.push((
                    SheetPlay { personnel: f.personnel, backfield: f.backfield, set: f.set, play: kind, hash },
                    1.0,
                ));
            }
            let want = if matches!(k, sheet::SCRIPT | sheet::SHOTS | sheet::TWO_POINT) {
                want
            } else {
                want.max(cover(&counts, COVER)).min(BOX_MAX)
            };
            let list = if k == sheet::SCRIPT && !raw.is_empty() { raw } else { mix(counts, want) };
            for (i, p) in list.into_iter().enumerate() {
                let mut marks = 0;
                if matches!(p.play, SheetKind::Pass(c) if favs.contains(&(c as usize))) {
                    marks |= FAV;
                }
                if family_know(team, family_of(&p)) < 0.72 {
                    marks |= CUT;
                }
                let why = off_reason(&p, sheet::key(k, hash), film, marks, &mut rng);
                let tests = if k == sheet::SCRIPT { tests_of(&p, i) } else { String::new() };
                off.push(Entry { k: sheet::key(k, hash), play: Play::Off(p), why, tests, marks });
            }
        }
    }
    // Tendency breakers (G12): the opposite of what a look gives away, on
    // first and second down.
    let take = if breakers { 3 } else { 2 };
    for t in tells.iter().take(take) {
        let f = formation_of(t.look);
        let pass = t.run_share() >= 0.5;
        let play = if pass {
            SheetKind::Pass(play_action_concept() as u16)
        } else {
            SheetKind::Run(crate::game::playbook::RunConcept::InsideZone)
        };
        let p = SheetPlay { personnel: f.personnel, backfield: f.backfield, set: f.set, play, hash: 0 };
        off.push(Entry {
            k: sheet::FIRST,
            play: Play::Off(p),
            why: off_reason(&p, sheet::FIRST, film, BREAKER, &mut rng),
            tests: String::new(),
            marks: BREAKER,
        });
    }
    // Your designs in the install go on the first-down list.
    for &(i, f, learned) in &inp.designs {
        let p = SheetPlay {
            personnel: f.personnel,
            backfield: f.backfield,
            set: f.set,
            play: SheetKind::Design(i),
            hash: 0,
        };
        let marks = if learned < 0.35 { CUT } else { 0 };
        off.push(Entry {
            k: sheet::FIRST,
            play: Play::Off(p),
            why: "our own play, going in this week".into(),
            tests: String::new(),
            marks,
        });
    }
    // The bad-weather sheet (G13): the downs and the red zone, runs up and
    // no deep shots.
    let wet: Vec<Entry> = off
        .iter()
        .filter(|e| matches!(sheet::split(e.k).0, 1..=7 | 10) && e.marks & BREAKER == 0)
        .filter(|e| match e.play {
            Play::Off(p) => match p.play {
                SheetKind::Pass(i) => pass_concept(i as usize).depth != PassDepth::Deep,
                SheetKind::Run(_) => true,
                SheetKind::Design(_) => false,
            },
            _ => false,
        })
        .map(|e| {
            let mut e = e.clone();
            e.marks |= WET;
            e.why = match e.play {
                Play::Off(p) if is_run(&p) => "in the weather: run it and keep the ball safe".into(),
                _ => "a short, sure throw for a wet ball".into(),
            };
            e
        })
        .collect();
    off.extend(wet);
    // The defense (G8).
    let mut def: Vec<Entry> = Vec::new();
    let (_, _, _, _, man_d, rush_d) = biases(notes[1]);
    let mut dstaff = staff;
    if man_d != 0.0 || rush_d != 0.0 {
        let own_man = guide::expected_man(staff.shell, &staff.guide);
        dstaff.guide.man = guide::Want::new(clamp(own_man + man_d, 0.05, 0.9), guide::Firm::Firm);
        dstaff.guide.pressure = guide::Want::new(
            clamp(guide::habit_pressure(staff.defense.aggressiveness) + rush_d, 0.05, 0.6),
            guide::Firm::Firm,
        );
    }
    let mut dmem = Memory::default();
    dmem.study(film);
    let looks = [
        (Formation::plain(Personnel::P11, Backfield::Shotgun, 1.0), 6.0),
        (Formation::plain(Personnel::P12, Backfield::UnderCenter, 1.0), 2.5),
        (Formation::plain(Personnel::P21, Backfield::UnderCenter, 1.0), 1.0),
        (Formation::plain(Personnel::P10, Backfield::Shotgun, 1.0), 0.8),
    ];
    let dboxes = [
        sheet::FIRST,
        sheet::SECOND_LONG,
        sheet::SECOND_MEDIUM,
        sheet::SECOND_SHORT,
        sheet::THIRD_LONG,
        sheet::THIRD_MEDIUM,
        sheet::THIRD_SHORT,
        sheet::RED_ZONE,
        sheet::GOAL_LINE,
        sheet::BACKED_UP,
        sheet::FOUR_MINUTE,
        sheet::TWO_MINUTE,
        sheet::SHOTS,
        sheet::TWO_POINT,
    ];
    for &k in &dboxes {
        let (sits, _) = box_spec(if k == sheet::SHOTS { sheet::THIRD_MEDIUM } else { k }, college, two_from);
        let n = if k == sheet::SHOTS { 4 } else { 3 };
        let mut counts: Vec<(DefSheetCall, f64)> = Vec::new();
        for i in 0..120 {
            let s = &sits[i % sits.len()];
            let f = rng.weighted(&looks);
            let d: DefCall = crate::game::calling::call_defense(&dstaff, &dmem, s, &f, &DefCtx::default(), &mut rng);
            if k == sheet::SHOTS && d.pressure.extra() == 0 {
                continue;
            }
            let key = (d.coverage, d.pressure);
            match counts.iter_mut().find(|x| x.0 == key) {
                Some(x) => x.1 += 1.0,
                None => counts.push((key, 1.0)),
            }
        }
        counts.sort_by(|a, b| b.1.total_cmp(&a.1));
        if counts.is_empty() {
            counts.push(((Coverage::Cover1, Pressure::LbBlitz), 1.0));
        }
        let n = if k == sheet::SHOTS { n } else { n.max(cover(&counts, COVER)).min(DBOX_MAX) };
        for ((c, p), _) in counts.into_iter().take(n) {
            def.push(Entry {
                k,
                play: Play::Def(c, p),
                why: def_reason(c, p, k, film),
                tests: String::new(),
                marks: 0,
            });
        }
    }
    // The defense's opening script (G11): eight calls for the first early
    // downs, each testing something on their offense.
    {
        let (sits, _) = box_spec(sheet::SCRIPT, college, two_from);
        let mut seen: Vec<DefSheetCall> = Vec::new();
        let mut script: Vec<DefSheetCall> = Vec::new();
        for i in 0..160 {
            if script.len() >= 8 {
                break;
            }
            let s = &sits[i % sits.len()];
            let f = rng.weighted(&looks);
            let d: DefCall = crate::game::calling::call_defense(&dstaff, &dmem, s, &f, &DefCtx::default(), &mut rng);
            let key = (d.coverage, d.pressure);
            // Variety first: each call once, then repeats.
            if !seen.contains(&key) || i >= 100 {
                seen.push(key);
                script.push(key);
            }
        }
        if script.is_empty() {
            script.push((Coverage::Cover3, Pressure::Four));
        }
        let tests = [
            "their first answer to our base look",
            "how they protect against five",
            "whether they run at our nickel",
            "their quick game against it",
            "how they handle a safety down",
            "their screens and draws",
            "whether they check out of it",
            "their tempo when we disguise",
        ];
        for (i, (c, p)) in script.into_iter().enumerate() {
            def.push(Entry {
                k: sheet::SCRIPT,
                play: Play::Def(c, p),
                why: def_reason(c, p, sheet::FIRST, film),
                tests: tests[i % tests.len()].to_string(),
                marks: 0,
            });
        }
    }
    // The defense in bad weather: the base downs with a safety down.
    let dwet: Vec<Entry> = def
        .iter()
        .filter(|e| matches!(e.k, 1..=6 | 10))
        .map(|e| {
            let mut e = e.clone();
            e.marks |= WET;
            e.why = "in the weather they'll run it: eight in the box".into();
            if let Play::Def(c, p) = e.play
                && c.high() == 2
            {
                e.play = Play::Def(Coverage::Cover3, p);
            }
            e
        })
        .collect();
    def.extend(dwet);
    (off, def)
}

/// A drafted offense as the engine reads it: each box's calls, dry or
/// wet. A design goes on once `design` gives its playbook id (once it is
/// in).
pub fn sheet_calls(off: &[Entry], wet: bool, design: impl Fn(u16) -> Option<usize>) -> Vec<(u8, Vec<SheetCall>)> {
    let mut v: Vec<(u8, Vec<SheetCall>)> = Vec::new();
    for e in off.iter().filter(|e| (e.marks & WET != 0) == wet) {
        let Play::Off(p) = e.play else { continue };
        let kind = match p.play {
            SheetKind::Run(r) => PlayKind::Run(r),
            SheetKind::Pass(i) => PlayKind::Pass(i as usize),
            SheetKind::Design(k) => match design(k) {
                Some(id) => PlayKind::Pass(id),
                None => continue,
            },
        };
        let call = (p.personnel, p.backfield, p.set, kind);
        match v.iter_mut().find(|x| x.0 == e.k) {
            Some(x) => x.1.push(call),
            None => v.push((e.k, vec![call])),
        }
    }
    v
}

/// A drafted defense as the engine reads it: each box's coverage and
/// pressure, dry or wet.
pub fn dsheet_calls(def: &[Entry], wet: bool) -> Vec<(u8, Vec<DefSheetCall>)> {
    let mut v: Vec<(u8, Vec<DefSheetCall>)> = Vec::new();
    for e in def.iter().filter(|e| (e.marks & WET != 0) == wet) {
        let Play::Def(c, p) = e.play else { continue };
        match v.iter_mut().find(|x| x.0 == e.k) {
            Some(x) => x.1.push((c, p)),
            None => v.push((e.k, vec![(c, p)])),
        }
    }
    v
}

/// Every other team's sheet (Cain's rule: every team plays by the same
/// rules): its coordinators draft it at kickoff the way yours do on plan
/// day, from the opponent's film, their own habits and their head coach's
/// guidelines, with the tendency breakers their own film calls for. It
/// lives for the game only (`game::flow`). Your team's comes from your
/// week (`prepare`).
pub fn kickoff_card(team: &GameTeam, inp: &DraftIn) -> sheet::Card {
    let (off, def) = draft_sides(team, inp, [&[], &[]]);
    let g = team.staff.guide;
    let pull = |own: f64| g.stick.pull().map_or(own, |(t, k)| guide::expected(own, t, k));
    let stick = pull(guide::habit_stick(team.staff.offense.creativity));
    let dstick = pull(guide::habit_stick(team.staff.defense.creativity));
    let none = |_: u16| None;
    let mut card = sheet::Card {
        sheet: sheet_calls(&off, false, none),
        wet: sheet_calls(&off, true, none),
        dsheet: dsheet_calls(&def, false),
        dwet: dsheet_calls(&def, true),
        ..Default::default()
    };
    card.stick = if card.sheet.is_empty() { 0.0 } else { stick };
    card.dstick = if card.dsheet.is_empty() { 0.0 } else { dstick };
    card
}

/// The tells in a team's own film, as a defense would read them and as
/// its staff self-scouts (G6, the same lines as yours): a look with 8
/// snaps or more where one kind is 75% or more of the calls, on film of
/// two games or more (about 65 snaps a game; proposed, not confirmed).
pub fn film_tells(own: &Film) -> Vec<Tell> {
    let games = (own.off_snaps / 65.0).floor() as u32;
    if games < 2 {
        return Vec::new();
    }
    let mut out: Vec<Tell> = own
        .look
        .iter()
        .enumerate()
        .map(|(look, x)| Tell { look, runs: x[0].round() as u32, passes: x[1].round() as u32, games })
        .filter(|t| t.snaps() >= 8 && (t.run_share() >= 0.75 || t.run_share() <= 0.25))
        .collect();
    out.sort_by_key(|x| std::cmp::Reverse(x.snaps()));
    out
}

fn deep_concept() -> usize {
    PASS_CONCEPTS.iter().position(|c| c.depth == PassDepth::Deep).unwrap_or(0)
}

fn play_action_concept() -> usize {
    PASS_CONCEPTS.iter().position(|c| c.play_action).unwrap_or(0)
}

fn formation_of(look: usize) -> Formation {
    use crate::game::playbook::{SET_COUNT, Set};
    let pers = [Personnel::P11, Personnel::P12, Personnel::P21, Personnel::P10, Personnel::P22, Personnel::P13];
    let p = pers.get(look / (3 * SET_COUNT)).copied().unwrap_or(Personnel::P11);
    let b = [Backfield::UnderCenter, Backfield::Shotgun, Backfield::Pistol][(look / SET_COUNT) % 3];
    let s = [Set::Doubles, Set::Trips, Set::Bunch, Set::Stack, Set::Empty, Set::Wing][look % SET_COUNT];
    let mut f = Formation::plain(p, b, 1.0);
    f.set = s;
    f
}

pub fn level_of(l: Level) -> crate::player::Level {
    match l {
        Level::Nfl | Level::Other => crate::player::Level::NFL,
        Level::HighSchool => crate::player::Level::HS,
        _ => crate::player::Level::FBS,
    }
}

/// Your team's play callers: the offensive and defensive (coach index).
pub fn callers(w: &World) -> (Option<usize>, Option<usize>) {
    let Some(s) = w.seat else { return (None, None) };
    let (head, oc, dc) = match s.level {
        Level::College => s.program.map(|p| &w.programs[p]).map_or((None, None, None), |p| (p.head_coach, p.oc, p.dc)),
        Level::Nfl => s.nfl_team.map(|t| &w.nfl[t]).map_or((None, None, None), |t| (t.head_coach, t.oc, t.dc)),
        Level::HighSchool => {
            let c = s.program.and_then(|h| w.high_schools[h].coach);
            (c, c, c)
        }
        Level::Other => {
            s.other.and_then(|t| w.others.teams.get(t)).map_or((None, None, None), |t| (t.head_coach, t.oc, t.dc))
        }
        _ => (None, None, None),
    };
    (super::season::play_caller(w, oc, head, true).or(head), super::season::play_caller(w, dc, head, false).or(head))
}

/// The drafted offense, as the practice week and the engine read the
/// sheet: your installs follow it.
fn sync(w: &mut World) {
    let Some(d) = w.plans.draft.as_ref() else { return };
    let mut sheet: Vec<(u8, Vec<SheetPlay>)> = Vec::new();
    for e in d.off.iter().filter(|e| e.marks & WET == 0) {
        let Play::Off(p) = e.play else { continue };
        let (k, _) = sheet::split(e.k);
        match sheet.iter_mut().find(|x| x.0 == k) {
            Some(x) => x.1.push(p),
            None => sheet.push((k, vec![p])),
        }
    }
    sheet.sort_by_key(|x| x.0);
    w.training.plan.sheet = sheet;
    w.training.plan.game = Some(d.game);
}

/// The draft for your next game, drafted now if the coordinators haven't
/// yet. Returns whether one is there.
pub fn ensure_draft(w: &mut World) -> bool {
    let Some(next) = next_game(w) else { return false };
    if w.plans.draft.as_ref().is_some_and(|d| d.game == next.date && d.opp == next.opp) {
        return true;
    }
    let Some(d) = draft_for(w, &next, [&[], &[]], 0) else { return false };
    w.plans.draft = Some(d);
    sync(w);
    w.refresh_seat_team();
    true
}

/// Your review of a side's draft: approve parts (None: all), or send it
/// back with notes, and the coordinator redrafts what you haven't approved.
pub fn review(w: &mut World, side: usize, part: Option<usize>, approve: bool, notes: Vec<u8>) -> Result<(), String> {
    if !ensure_draft(w) {
        return Err("No game ahead to plan for.".into());
    }
    let side = side.min(1);
    if approve {
        let d = w.plans.draft.as_mut().ok_or("No draft.")?;
        for (i, s) in d.status[side].iter_mut().enumerate() {
            if part.is_none_or(|p| p == i) {
                *s = 1;
            }
        }
        return Ok(());
    }
    let next = next_game(w).ok_or("No game ahead.")?;
    let old = w.plans.draft.clone().ok_or("No draft.")?;
    let mut all = old.notes.clone();
    all[side] = notes.clone();
    let round = old.round.saturating_add(1);
    let mut fresh =
        draft_for(w, &next, [&all[0], &all[1]], round).ok_or("Your coordinators can't draft without a team.")?;
    // The coordinator redraws the part sent back (every part not approved
    // when none is named). Approved parts, the other parts, the other side
    // and the plays you added yourself stay as they were.
    let redo = |p: usize, s: usize| s == side && old.status[s][p] != 1 && part.is_none_or(|x| x == p);
    let merge = |old_list: &[Entry], fresh_list: Vec<Entry>, s: usize| -> Vec<Entry> {
        let mut v: Vec<Entry> =
            old_list.iter().filter(|e| !redo(part_of(e.k), s) || e.marks & YOURS != 0).cloned().collect();
        for e in fresh_list.into_iter().filter(|e| redo(part_of(e.k), s)) {
            let dup = v.iter().any(|x| x.k == e.k && x.play == e.play && (x.marks & WET) == (e.marks & WET));
            if !dup {
                v.push(e);
            }
        }
        v
    };
    fresh.off = merge(&old.off, std::mem::take(&mut fresh.off), 0);
    fresh.def = merge(&old.def, std::mem::take(&mut fresh.def), 1);
    fresh.status = old.status;
    fresh.edits = old.edits;
    let list = if side == 0 { &fresh.off } else { &fresh.def };
    for p in 0..fresh.status[side].len() {
        if redo(p, side) {
            fresh.status[side][p] = 2;
            // Your edits there went with the old draft; the plays you
            // added are what is left of them.
            let yours = list.iter().filter(|e| part_of(e.k) == p && e.marks & YOURS != 0).count();
            fresh.edits[side][p] = yours.min(u8::MAX as usize) as u8;
        }
    }
    fresh.drafted = old.drafted;
    w.plans.draft = Some(fresh);
    sync(w);
    w.refresh_seat_team();
    Ok(())
}

/// Your edit of one line: keep it, move it up, cut it (a proposed cut is
/// replaced by the coordinator's next play for the box), or add a play.
pub fn edit(w: &mut World, side: usize, i: usize, act: &str, add: Option<Play>) -> Result<(), String> {
    if !ensure_draft(w) {
        return Err("No game ahead to plan for.".into());
    }
    let d = w.plans.draft.as_mut().ok_or("No draft.")?;
    let list = if side == 0 { &d.off } else { &d.def };
    if act == "add" {
        let play = add.ok_or("Pick a play.")?;
        let k = list.get(i).map_or(sheet::FIRST, |e| e.k);
        let wet = list.get(i).map_or(0, |e| e.marks & WET);
        return add_to_box(w, side, k, wet, play);
    }
    let list = if side == 0 { &mut d.off } else { &mut d.def };
    {
        let e = list.get(i).ok_or("No such play.")?.clone();
        let part = part_of(e.k);
        match act {
            "up" => {
                let prev = (0..i).rev().find(|&j| list[j].k == e.k && (list[j].marks & WET) == (e.marks & WET));
                let Some(j) = prev else { return Ok(()) };
                list.swap(i, j);
            }
            "cut" => {
                list.remove(i);
            }
            "keep" => {
                list[i].marks &= !CUT;
            }
            _ => return Err("Keep, move up, cut or add.".into()),
        }
        let n = &mut d.edits[side.min(1)][part];
        *n = n.saturating_add(1);
        d.status[side.min(1)][part] = 2;
    }
    sync(w);
    w.refresh_seat_team();
    Ok(())
}

/// A design deleted from the play designer: it leaves the drafted sheet,
/// and the designs after it move down one, as they do on the practice
/// sheet and the installs.
pub fn design_removed(w: &mut World, k: u16) {
    let Some(d) = w.plans.draft.as_mut() else { return };
    d.off.retain(|e| !matches!(e.play, Play::Off(p) if p.play == SheetKind::Design(k)));
    for e in d.off.iter_mut() {
        if let Play::Off(p) = &mut e.play
            && let SheetKind::Design(x) = p.play
            && x > k
        {
            p.play = SheetKind::Design(x - 1);
        }
    }
}

/// Your play added to a box (`k` with its hash: 0 either hash; `wet` the
/// bad-weather sheet's bit).
pub fn add_to_box(w: &mut World, side: usize, k: u8, wet: u8, play: Play) -> Result<(), String> {
    if !ensure_draft(w) {
        return Err("No game ahead to plan for.".into());
    }
    let d = w.plans.draft.as_mut().ok_or("No draft.")?;
    let list = if side == 0 { &mut d.off } else { &mut d.def };
    let count = list.iter().filter(|e| e.k == k && e.marks & WET == wet).count();
    if count >= BOX_MAX {
        return Err("Thirty plays a box at most: a call sheet is one card.".into());
    }
    list.push(Entry { k, play, why: "your call".into(), tests: String::new(), marks: YOURS | wet });
    let n = &mut d.edits[side.min(1)][part_of(k)];
    *n = n.saturating_add(1);
    d.status[side.min(1)][part_of(k)] = 2;
    sync(w);
    w.refresh_seat_team();
    Ok(())
}

/// The week's plan for your next game, reset when the game changes.
pub fn week_mut(w: &mut World) -> &mut Week {
    let game = next_game(w).map(|n| n.date);
    if w.plans.week.game != game {
        w.plans.week = Week { game, ..Week::default() };
    }
    &mut w.plans.week
}

/// The staff's own quarterback checks for an opponent (G19): the two
/// usual ones, the hot throw against a team that plays Cover 0, the run
/// against two-high against a team that lives in it.
pub fn staff_checks(film: &Film) -> u8 {
    crate::game::roster::staff_checks(film)
}

/// The staff's own "if they do this" lines for an opponent (G20): the ones
/// their film says could come up (the engine reads every AI staff's the
/// same way, `game::roster::staff_ifs`).
pub fn staff_ifs(film: &Film) -> Vec<u8> {
    crate::game::roster::staff_ifs(film)
}

/// Who knows our call words (G5): people who were with us since the
/// words were set and are now somewhere else.
#[derive(Clone, Debug, PartialEq)]
pub struct Knower {
    pub name: String,
    pub what: String,
    /// 0 our offense's words, 1 the defense's.
    pub side: usize,
    pub now: String,
    /// Their team now, at your level.
    pub team: Option<usize>,
    pub share: f64,
    pub person: Option<usize>,
    pub coach: Option<usize>,
}

pub fn knowers(w: &World) -> Vec<Knower> {
    let Some((level, t)) = w.seat_team() else { return Vec::new() };
    let since = |side: usize| if w.plans.words.since[side] > 0 { w.plans.words.since[side] } else { first_year(w) };
    let mut out = Vec::new();
    match level {
        Level::College => {
            let roster = &w.programs[t].roster;
            for (i, p) in w.people.iter().enumerate() {
                // The cheap test first: almost nobody was ever ours.
                if !p.college_staff.iter().any(|s| s.1 == t) || roster.contains(&i) {
                    continue;
                }
                let side = if super::systems::offense(p.player.position) { 0 } else { 1 };
                let last = p.college_staff.iter().filter(|s| s.1 == t).map(|s| s.0).max().unwrap_or(0);
                if last < since(side) {
                    continue;
                }
                let (now, team) = match p.stage {
                    super::Stage::College => match p.college {
                        Some(c) if c != t => (w.program_name(c).to_string(), Some(c)),
                        _ => continue,
                    },
                    super::Stage::Pro => (pro_team(w, p), None),
                    _ => continue,
                };
                let share = if p.last_role == Some(super::PlayingTime::Starter) { 1.0 } else { 0.7 };
                out.push(Knower {
                    name: p.name.clone(),
                    what: p.player.position.name().to_string(),
                    side,
                    now,
                    team,
                    share,
                    person: Some(i),
                    coach: None,
                });
            }
        }
        Level::Nfl => {
            let roster = &w.clubs[t].roster;
            for (i, p) in w.people.iter().enumerate() {
                let Some(c) = p.pro.as_ref() else { continue };
                if roster.contains(&i) || c.team == Some(t) {
                    continue;
                }
                let side = if super::systems::offense(p.player.position) { 0 } else { 1 };
                let last = c.seasons.iter().filter(|s| s.team == Some(t)).map(|s| s.year).max();
                if last.is_none_or(|y| y < since(side)) {
                    continue;
                }
                let Some(now) = c.team else { continue };
                let starts: u32 = c.seasons.iter().filter(|s| s.team == Some(t)).map(|s| s.starts).sum();
                out.push(Knower {
                    name: p.name.clone(),
                    what: p.player.position.name().to_string(),
                    side,
                    now: w.nfl.get(now).map_or(String::new(), |x| x.name.to_string()),
                    team: Some(now),
                    share: if starts >= 8 { 1.0 } else { 0.7 },
                    person: Some(i),
                    coach: None,
                });
            }
        }
        _ => {}
    }
    // Coaches who left the staff with the words: a coach knows his own
    // side's words (new words on that side close him out).
    for (i, c) in w.coaches.iter().enumerate() {
        let side = if c.came_up.offense() { 0 } else { 1 };
        let was = c.history.iter().any(|s| {
            let ours = match (level, s.employer) {
                (Level::College, super::coach::Employer::College(p)) => p == t,
                (Level::Nfl, super::coach::Employer::Nfl(p)) => p == t,
                (Level::HighSchool, super::coach::Employer::HighSchool(p)) => p == t,
                _ => false,
            };
            ours && s.to >= since(side)
        });
        if !was || c.retired || c.yours.is_some() {
            continue;
        }
        let Some(job) = c.job else { continue };
        let (now, team) = match (level, job.employer) {
            (Level::College, super::coach::Employer::College(p)) if p != t => (w.program_name(p).to_string(), Some(p)),
            (Level::Nfl, super::coach::Employer::Nfl(p)) if p != t => {
                (w.nfl.get(p).map_or(String::new(), |x| x.name.to_string()), Some(p))
            }
            (Level::HighSchool, super::coach::Employer::HighSchool(p)) if p != t => {
                (w.school_name(p).to_string(), Some(p))
            }
            (_, super::coach::Employer::College(p)) => (w.program_name(p).to_string(), None),
            (_, super::coach::Employer::Nfl(p)) => (w.nfl.get(p).map_or(String::new(), |x| x.name.to_string()), None),
            _ => continue,
        };
        out.push(Knower {
            name: c.name.clone(),
            what: "coach".into(),
            side,
            now,
            team,
            share: 1.0,
            person: None,
            coach: Some(i),
        });
    }
    out
}

/// The year you came to this team (your call words are as old as your job
/// here at most).
fn first_year(w: &World) -> i32 {
    w.career.as_ref().map_or(w.year, |c| c.market.job_since.unwrap_or(c.started).year())
}

fn pro_team(w: &World, p: &super::Person) -> String {
    p.pro.as_ref().and_then(|c| c.team).and_then(|t| w.nfl.get(t)).map_or("the NFL".into(), |t| t.name.to_string())
}

/// How much of our calls this week's opponent reads (G5): each knower on
/// their team or staff reads about one call in nine on his side, a backup
/// less; at most three in ten.
pub fn leak(w: &World, opp: usize) -> [f64; 2] {
    leak_from(&knowers(w), opp)
}

/// The same from the knowers in hand (a board that lists them).
pub fn leak_from(knowers: &[Knower], opp: usize) -> [f64; 2] {
    let mut v = [0.0f64; 2];
    for k in knowers.iter().filter(|k| k.team == Some(opp)) {
        v[k.side] += 0.11 * k.share;
    }
    [v[0].min(0.3), v[1].min(0.3)]
}

/// New words cost a little learning (G5): about one extra bust in 70 snaps
/// for the side that changed, the week of the change.
pub const WORDS_LEARN: f64 = 1.0 / 770.0;

/// Your game team takes the deeper plan (run from `training::seat::prepare`
/// after your practice week): the sheets, how closely the play callers
/// stick to them, the words, the week's plan against this opponent.
pub(crate) fn prepare(w: &World, team: &mut GameTeam) {
    let next = next_game(w);
    let g = team.staff.guide;
    // Stick to the card (G1, the program's guideline).
    let own = guide::habit_stick(team.staff.offense.creativity);
    let stick = g.stick.pull().map_or(own, |(t, k)| guide::expected(own, t, k));
    // The drafted sheets for this game (stored, or as the coordinators
    // would draft them now: the same draft, keyed by the game).
    let drafted = match (next, w.plans.draft.as_ref()) {
        (Some(n), Some(d)) if d.game == n.date && d.opp == n.opp => Some(d.clone()),
        (Some(n), _) => Some(draft_with(w, team, &n, [&[], &[]], 0)),
        _ => None,
    };
    let prep = &mut team.prep;
    if let Some(d) = drafted.as_ref() {
        // A design goes on the card once the players know it a third of
        // the way.
        let t = &w.training;
        let design = |k: u16| {
            if t.install_level(crate::training::seat::InstallKey::Design(k)) < 0.35 {
                return None;
            }
            t.design_ids.get(k as usize).copied()
        };
        prep.sheet = sheet_calls(&d.off, false, design);
        prep.wet = sheet_calls(&d.off, true, design);
        prep.dsheet = dsheet_calls(&d.def, false);
        prep.dwet = dsheet_calls(&d.def, true);
    }
    prep.stick = if prep.sheet.is_empty() { 0.0 } else { stick };
    // The same guideline holds the defensive play caller (G1: both callers).
    let own_d = guide::habit_stick(team.staff.defense.creativity);
    let dstick = g.stick.pull().map_or(own_d, |(t, k)| guide::expected(own_d, t, k));
    prep.dstick = if prep.dsheet.is_empty() { 0.0 } else { dstick };
    // The words (G5).
    let words = w.plans.words;
    prep.learn = [0, 1].map(|s| {
        let fresh = match (words.changed[s], next) {
            (Some(c), Some(n)) => n.date.0 - c.0 <= 10 && n.date.0 >= c.0,
            _ => false,
        };
        if fresh { WORDS_LEARN } else { 0.0 }
    });
    prep.leak = next.map_or([0.0; 2], |n| leak(w, n.opp));
    // Depth and rotation (G15): the season's.
    prep.backups = w.plans.depth.iter().map(|(b, v)| (*b as u8, v.clone())).collect();
    prep.rotation = w.plans.rotation.clone();
    // The week against this opponent (G16-G20).
    let wk = w.plans.week.clone();
    let this = next.is_some_and(|n| wk.game == Some(n.date));
    let film = next.map(|n| opp_film(w, n.level, n.opp)).unwrap_or_default();
    prep.answers = if this { wk.answers } else { Default::default() };
    prep.limits = if this { wk.limits.clone() } else { Vec::new() };
    prep.teams = if this { wk.teams } else { Teams::default() };
    prep.checks = wk.checks.filter(|_| this).unwrap_or_else(|| staff_checks(&film));
    prep.ifs = wk.ifs.clone().filter(|_| this).unwrap_or_else(|| staff_ifs(&film));
    prep.planned = true;
    prep.base_only = if this { wk.base_only.clone() } else { Vec::new() };
    // Instructions for this opponent (G19): take back last time's, give
    // this week's.
    let old = std::mem::take(&mut prep.orders);
    let new: Vec<(u32, Instruction)> = if this {
        wk.orders.iter().filter_map(|&(id, k)| ORDERS.get(k as usize).map(|o| (id, o.0))).collect()
    } else {
        Vec::new()
    };
    let old_boosts = std::mem::take(&mut prep.boosts);
    let old_added = std::mem::take(&mut prep.boost_added);
    let boosts: Vec<(u32, f32)> = if this {
        wk.extra.iter().map(|&(id, x)| (id, if x == 1 { 0.04 } else { 0.03 })).collect()
    } else {
        Vec::new()
    };
    // Only what this plan gave is taken back: an instruction the staff had
    // already given stays, and the know added is what the clamp let in.
    let mut pushed: Vec<(u32, Instruction)> = Vec::new();
    let mut added: Vec<(u32, Vec<f32>)> = Vec::new();
    for p in team.players.iter_mut() {
        for (_, ins) in old.iter().filter(|o| o.0 == p.id) {
            if let Some(i) = p.orders.0.iter().position(|x| x == ins) {
                p.orders.0.remove(i);
            }
        }
        for (_, ins) in new.iter().filter(|o| o.0 == p.id) {
            if !p.orders.0.contains(ins) {
                p.orders.0.push(*ins);
                pushed.push((p.id, *ins));
            }
        }
        match old_added.iter().find(|b| b.0 == p.id) {
            Some((_, d)) => {
                for (k, x) in p.know.iter_mut().zip(d) {
                    *k = (*k - x).clamp(0.0, 1.0);
                }
            }
            // A save from before kept only the amount.
            None => {
                let back: f32 = old_boosts.iter().filter(|b| b.0 == p.id).map(|b| b.1).sum();
                if back != 0.0 {
                    for k in p.know.iter_mut() {
                        *k = (*k - back).clamp(0.0, 1.0);
                    }
                }
            }
        }
        let add: f32 = boosts.iter().filter(|b| b.0 == p.id).map(|b| b.1).sum();
        if add != 0.0 {
            let d: Vec<f32> = p
                .know
                .iter_mut()
                .map(|k| {
                    let was = *k;
                    *k = (*k + add).clamp(0.0, 1.0);
                    *k - was
                })
                .collect();
            added.push((p.id, d));
        }
    }
    team.prep.orders = pushed;
    team.prep.boosts = boosts;
    team.prep.boost_added = added;
}

/// The day's planning (from the clock): the coordinators draft the next
/// game's sheet on plan day, the keys of a game just played are checked,
/// and the identity's firm guidelines weigh on the coordinators who don't
/// share them, a little each month.
pub(crate) fn day(w: &mut World, d: Date) {
    if w.seat.is_none() || w.career.is_none() {
        return;
    }
    // Your high school's summer camp install (G2).
    super::hs_year::camp_install(w);
    // The keys of a game just played, checked before a short week's plan
    // day puts the next game's on the board (a Thursday game's plan day is
    // the Saturday before: the day after a Sunday game, or that day).
    check_keys(w);
    if let Some(n) = next_game(w)
        && d >= plan_day(n.level, n.date)
        && w.plans.draft.as_ref().is_some_and(|x| x.game == n.date && x.opp == n.opp && x.early_look(n.level))
    {
        // An early look nobody touched: plan day drafts it again.
        w.plans.draft = None;
    }
    if let Some(n) = next_game(w)
        && d >= plan_day(n.level, n.date)
    {
        if !w.plans.draft.as_ref().is_some_and(|x| x.game == n.date && x.opp == n.opp) {
            ensure_draft(w);
        }
        // The keys go on the board on plan day whoever drafted the sheet,
        // a sheet approved before plan day included.
        if w.plans.week.game != Some(n.date) || w.plans.week.keys.is_empty() {
            set_keys(w);
        }
    }
    // The team's grasp of the plan today (G14), for the week's chart.
    if let Some(n) = next_game(w).filter(|n| n.date.0 - d.0 <= 8)
        && let Some(team) = our_team(w)
    {
        let fams = sheet_families(w);
        let g = [0, 1, 2].map(|u| unit_grasp(&team, n.level, u, &fams) as f32);
        w.plans.grasp_log.retain(|x| x.0 != d && d.0 - x.0.0 <= 14);
        w.plans.grasp_log.push((d, g));
    }
    // Trust (G1): a coordinator held firmly to what he doesn't believe in.
    let month = (d.year(), d.month() as u8);
    if w.plans.trust_month != Some(month) {
        w.plans.trust_month = Some(month);
        for (c, n) in firm_against(w) {
            let talked = w.plans.talks.iter().any(|t| t.0 == c && d.0 - t.1.0 <= 31);
            let by = if talked { -0.004 } else { -0.01 } * n as f32;
            w.folk.mine.nudge(super::folk::coach_key(c), by);
        }
    }
}

/// Coordinators and the number of firm guidelines against their own
/// leanings.
pub fn firm_against(w: &World) -> Vec<(usize, usize)> {
    if !w.seat_head() {
        return Vec::new();
    }
    let (oc, dc) = callers(w);
    let g = w.plans.guide;
    let mut out = Vec::new();
    for (c, offense) in [(oc, true), (dc, false)] {
        let Some(c) = c else { continue };
        if w.coaches[c].yours.is_some() {
            continue;
        }
        let n = disagreements(w, c, &g, offense).into_iter().filter(|x| x.1 == guide::Firm::Firm).count();
        if n > 0 {
            out.push((c, n));
        }
    }
    out
}

/// A coordinator's view of the guidelines on his side: (guideline key,
/// firmness) for each one he wouldn't pick himself.
pub fn disagreements(w: &World, c: usize, g: &Guidelines, offense: bool) -> Vec<(&'static str, guide::Firm)> {
    let own = super::guide::head_guidelines(w, Some(c), w.seat.map_or(Level::College, |s| s.level));
    let know = &w.coaches[c].systems.know;
    let mut v = Vec::new();
    if offense {
        if let Some(s) = g.system.to {
            let i = super::systems::OFFENSES.iter().position(|&o| o == s).unwrap_or(3);
            if know[i] < 0.6 {
                v.push(("system", g.system.firm));
            }
        }
        if let (Some(r), Some(o)) = (g.run.to, own.run.to.or(Some(0.42)))
            && (r - o).abs() > 0.1
        {
            v.push(("run", g.run.firm));
        }
        if let Some(p) = g.pace.to
            && p == guide::Pace::NoHuddle
            && own.pace.to != Some(guide::Pace::NoHuddle)
        {
            v.push(("pace", g.pace.firm));
        }
    } else {
        if let Some(f) = g.front.to {
            let i = super::systems::FRONTS.iter().position(|&x| x == f.front()).unwrap_or(0);
            if know[5 + i] < 0.6 {
                v.push(("front", g.front.firm));
            }
        }
        if let Some(m) = g.man.to {
            let best = (7..11).max_by(|&a, &b| know[a].total_cmp(&know[b])).unwrap_or(7);
            if (m - guide::habit_man(super::systems::SHELLS[best - 7])).abs() > 0.15 {
                v.push(("man", g.man.firm));
            }
        }
        if let Some(p) = g.pressure.to
            && (p - guide::habit_pressure(w.coaches[c].personality.ambition)).abs() > 0.1
        {
            v.push(("pressure", g.pressure.firm));
        }
    }
    v
}

/// The keys to the next game (G21), set on plan day from the opponent's
/// film: four, the way a staff writes them on the board.
fn set_keys(w: &mut World) {
    let Some(n) = next_game(w) else { return };
    let film = opp_film(w, n.level, n.opp);
    let ypc = match n.level {
        Level::Nfl | Level::Other => 4.2,
        Level::HighSchool => 4.4,
        _ => 4.6,
    };
    let blitz = film.blitz as f64 / film.def_snaps.max(1.0) as f64;
    let mut keys = vec![
        Key { kind: 0, bar: 0.5, text: "Win first down: 4 yards or more on half our first downs".into() },
        Key { kind: 1, bar: 1.0, text: "One turnover or fewer".into() },
        Key { kind: 2, bar: ypc as f32, text: format!("Hold them under {ypc:.1} yards a carry") },
    ];
    keys.push(if blitz >= 0.3 {
        Key { kind: 3, bar: 5.5, text: "Beat the blitz: 5.5 yards a play when they bring it".into() }
    } else {
        Key { kind: 4, bar: 0.4, text: "Convert 40% on third down".into() }
    });
    let wk = week_mut(w);
    wk.keys = keys;
}

/// Check the keys of games played since the last check.
fn check_keys(w: &mut World) {
    let Some((level, t)) = w.seat_team() else { return };
    let last = w.plans.keys.last().map(|k| k.date);
    let Some(g) = w.seat_games().last() else { return };
    if last.is_some_and(|d| d >= g.date) || w.plans.week.game != Some(g.date) || w.plans.week.keys.is_empty() {
        return;
    }
    let side = if g.teams[0] == t { 0 } else { 1 };
    let opp = team_name(w, level, g.teams[1 - side]);
    let plays: Vec<&crate::game::PlayRecord> = g.stats.plays.iter().filter(|r| !r.nullified).collect();
    let ours = |r: &&&crate::game::PlayRecord| {
        r.offense == side && matches!(r.kind, PlayType::Run | PlayType::Pass | PlayType::Sack | PlayType::Scramble)
    };
    let theirs = |r: &&&crate::game::PlayRecord| r.offense != side && r.kind == PlayType::Run;
    let mut out = Vec::new();
    for k in &w.plans.week.keys {
        let (met, what) = match k.kind {
            0 => {
                let f: Vec<_> = plays.iter().filter(ours).filter(|r| r.down == 1).collect();
                let good = f.iter().filter(|r| r.yards >= 4.0).count();
                let share = good as f64 / f.len().max(1) as f64;
                (share >= k.bar as f64, format!("{good} of {} first downs went for 4 or more", f.len()))
            }
            1 => {
                let n = plays.iter().filter(|r| r.offense == side && (r.interception || r.fumble_lost)).count();
                (n as f32 <= k.bar, format!("{n} {}", crate::world::folk::noun(n, "turnover")))
            }
            2 => {
                let r: Vec<_> = plays.iter().filter(theirs).collect();
                let ypc = r.iter().map(|x| x.yards).sum::<f64>() / r.len().max(1) as f64;
                (
                    ypc < k.bar as f64,
                    format!("{ypc:.1} a carry on {} {}", r.len(), crate::world::folk::noun(r.len(), "carry")),
                )
            }
            3 => {
                let b: Vec<_> = plays.iter().filter(ours).filter(|r| r.blitz).collect();
                let y = b.iter().map(|x| x.yards).sum::<f64>() / b.len().max(1) as f64;
                (
                    y >= k.bar as f64,
                    format!(
                        "{y:.1} a play on {} {} against the blitz",
                        b.len(),
                        crate::world::folk::noun(b.len(), "snap")
                    ),
                )
            }
            _ => {
                let th: Vec<_> = plays.iter().filter(ours).filter(|r| r.down == 3).collect();
                let conv = th.iter().filter(|r| r.first_down || r.touchdown).count();
                let share = conv as f64 / th.len().max(1) as f64;
                (share >= k.bar as f64, format!("{conv} of {} on third down", th.len()))
            }
        };
        out.push((k.text.clone(), met, what));
    }
    w.plans.keys.push(KeysCheck { date: g.date, opp, keys: out });
    if w.plans.keys.len() > 6 {
        w.plans.keys.remove(0);
    }
}

/// Grasp of the plan (G14), 0-1, for one player: how well he knows the
/// families on this week's sheet for his side, weighted by how much each
/// is called, and a share by Friday from the rest of the week's teaching.
pub fn grasp(team: &GameTeam, id: u32, off: bool, weights: &[(Family, f64)]) -> Option<f64> {
    let p = team.players.iter().find(|p| p.id == id)?;
    let fams: Vec<(Family, f64)> = weights.iter().copied().filter(|(f, _)| f.offense() == off).collect();
    let all: f64 = fams.iter().map(|x| x.1).sum();
    if all <= 0.0 {
        return Some(p.know.iter().copied().map(f64::from).sum::<f64>() / p.know.len() as f64);
    }
    Some(fams.iter().map(|(f, w)| w * p.know[*f as usize] as f64).sum::<f64>() / all)
}

/// The team's order at every position as the engine dresses it: your
/// order where you set one, your staff's for the rest (`team.order` holds
/// only what you set). Two-way play only at high school, as on game day.
pub fn depth_order(team: &GameTeam, level: Level) -> Vec<(crate::positions::Position, Vec<u32>)> {
    let margin = if level == Level::HighSchool { 4.0 } else { 99.0 };
    let d = crate::game::roster::depth_chart(team, margin);
    crate::positions::Position::ALL
        .iter()
        .map(|&pos| (pos, d.list(pos).iter().map(|&i| team.players[i].id).collect()))
        .collect()
}

/// A unit's grasp of the plan (G14): 0 offense, 1 defense, 2 the kicking
/// game, the mean over the unit's top players by the depth order.
pub fn unit_grasp(team: &GameTeam, level: Level, unit: usize, weights: &[(Family, f64)]) -> f64 {
    let mut v: Vec<f64> = Vec::new();
    for (pos, ids) in &depth_order(team, level) {
        let off = super::systems::offense(*pos);
        let fits = match unit {
            0 => off && !pos.is_specialist(),
            1 => !off && !pos.is_specialist(),
            _ => pos.is_specialist(),
        };
        if !fits {
            continue;
        }
        for &id in ids.iter().take(3) {
            if let Some(g) = grasp(team, id, unit != 1, weights) {
                v.push(g);
            }
        }
    }
    v.iter().sum::<f64>() / v.len().max(1) as f64
}

/// The engine's chance a player busts a snap from what he knows (the
/// scrimmage's own terms, without the snap's noise and fatigue).
pub fn bust_rate(know: f64, iq: f64, load: f64, prep: f64) -> f64 {
    ((1.0 - know) * 0.05 + (60.0 - iq).max(0.0) * 0.0006 + 0.04 * (load - crate::game::roster::absorb(iq)).max(0.0))
        .max(0.0)
        * prep
}

/// This week's sheet's families and how often each is on it (G14).
pub fn sheet_families(w: &World) -> Vec<(Family, f64)> {
    let mut v: Vec<(Family, f64)> = Vec::new();
    let mut add = |f: Family, x: f64| match v.iter_mut().find(|y| y.0 == f) {
        Some(y) => y.1 += x,
        None => v.push((f, x)),
    };
    if let Some(d) = w.plans.draft.as_ref() {
        for e in d.off.iter().filter(|e| e.marks & WET == 0) {
            if let Play::Off(p) = e.play {
                add(family_of(&p), 1.0);
            }
        }
        for e in d.def.iter().filter(|e| e.marks & WET == 0) {
            if let Play::Def(c, p) = e.play {
                add(
                    if c.is_man() {
                        Family::Man
                    } else if c.high() == 2 {
                        Family::TwoHigh
                    } else {
                        Family::SingleHigh
                    },
                    1.0,
                );
                if p.extra() > 0 {
                    add(Family::Pressure, 0.5);
                }
                add(Family::Front, 0.5);
            }
        }
    }
    v
}

impl Draft {
    /// A draft made before its plan day that you haven't approved, edited
    /// or sent back: an early look, which your coordinators draw up again
    /// on plan day from the latest film and roster.
    pub fn early_look(&self, level: Level) -> bool {
        self.drafted < plan_day(level, self.game)
            && self.status.iter().flatten().all(|&s| s == 0)
            && self.edits.iter().flatten().all(|&e| e == 0)
            && self.notes.iter().all(Vec::is_empty)
    }

    /// Every box on both sides has its calls, the opening scripts
    /// included: a drafted sheet is never empty.
    pub fn every_box_filled(&self) -> bool {
        OFF_BOXES.iter().all(|&k| {
            self.off.iter().any(|e| sheet::split(e.k).0 == k) && self.def.iter().any(|e| sheet::split(e.k).0 == k)
        })
    }
}
