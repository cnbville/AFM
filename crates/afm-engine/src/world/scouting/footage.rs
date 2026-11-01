//! What the tape holds (update 2, SK1-SK6): every staff in the world reads
//! a player through the same evidence, kept by the season records
//! (`world/records.rs`) and his high school lines. A look can show only
//! what he played (his snaps set how far film narrows a read, SK1), against
//! whom (dominance against weak opponents reads high until he's seen against
//! good players, SK2), what his role asked of him (a skill his role rarely
//! tested stays unseen, SK3), who was around him (a pass-happy system or a
//! great line flatters him to readers who lean on numbers, SK4), and how
//! the tape came in (a recruit's own reel shows only his best snaps, SK5).
//! The analytics staff reads his numbers instead (SK6).
//!
//! Every term here comes from what the world shows (snaps, schedules,
//! systems, teammates), so the whole league misses the same players for
//! the same visible reasons. Sizes are Claude's, proposed, not confirmed.

use std::collections::HashMap;

use crate::attributes::{Attr, Category};
use crate::positions::Position;
use crate::rng::clamp;
use crate::world::data::Division;
use crate::world::records::Stat;
use crate::world::{Stage, World};

/// The roles a snap can test.
pub const PASS_PRO: usize = 0;
pub const RUN_BLOCK: usize = 1;
pub const PASS_RUSH: usize = 2;
pub const MAN: usize = 3;
pub const ZONE: usize = 4;
pub const ROUTES: usize = 5;
pub const DROPBACKS: usize = 6;
pub const CARRIES: usize = 7;
pub const KICKS: usize = 8;
pub const ROLES: usize = 9;
/// The snaps a role needs before film shows the skills it tests.
pub const ROLE_MIN: f64 = 60.0;

/// What a player's tape holds.
#[derive(Clone, Copy, Debug, Default)]
pub struct Footage {
    /// 0 none kept, 1 high school, 2 college.
    pub level: u8,
    /// Film snaps at his level.
    pub snaps: f64,
    /// Starter seasons' worth of those snaps.
    pub starts: f64,
    /// Snaps by role, and which roles the record counts.
    pub role: [f64; ROLES],
    pub known: u16,
    /// His opponents against a typical FBS schedule, in rating points
    /// (below zero is weaker).
    pub comp: f64,
    /// How far he stood over them (0 to 1.5).
    pub dom: f64,
    /// How much his system and teammates lift his numbers (about -2 to 2).
    pub context: f64,
    /// His last season's play against what he is (form), rating points.
    pub luck: f64,
    /// A recruit in the reel era (Hudl, from about 2008).
    pub reels: bool,
    /// His production grade (0-100, 60 average), college only.
    pub grade: Option<f64>,
    pub division: Option<Division>,
}

fn div_offset(d: Division) -> f64 {
    match d {
        Division::FBS => 0.0,
        Division::FCS => -4.0,
        Division::D2 | Division::JUCO => -8.0,
        Division::NAIA => -9.0,
        Division::D3 => -10.0,
    }
}

/// A man-coverage shell's share of man snaps.
fn man_share(w: &World, prog: Option<usize>) -> f64 {
    use crate::game::roster::Shell;
    match prog.map(|c| w.programs[c].installed.shell()) {
        Some(Shell::ManPressure) => 0.7,
        Some(Shell::Quarters) => 0.35,
        Some(Shell::Cover3) => 0.25,
        Some(Shell::Cover2) => 0.2,
        None => 0.35,
    }
}

/// His tape, rebuilt from the records when a read asks.
pub fn footage(w: &World, i: usize) -> Footage {
    let p = &w.people[i];
    let pos = p.cal.pro_position.unwrap_or(p.player.position);
    let mut f = Footage {
        luck: clamp(p.last_performance.map_or(0.0, |x| x - p.player.overall()), -8.0, 8.0),
        ..Default::default()
    };
    match p.stage {
        Stage::College | Stage::Declared => college(w, i, pos, &mut f),
        Stage::HighSchool | Stage::PrepSchool => high_school(w, i, pos, &mut f),
        _ => {}
    }
    f
}

fn college(w: &World, i: usize, pos: Position, f: &mut Footage) {
    let p = &w.people[i];
    let recs = w.stats.career(p.player.id);
    f.level = 2;
    f.division = p.college.map(|c| w.programs[c].division);
    if recs.is_empty() {
        // Seasons before the book: his snaps only.
        f.snaps = p.college_snaps as f64;
        f.starts = f.snaps / 700.0;
        f.comp = f.division.map_or(0.0, div_offset);
        return;
    }
    let kicks = pos.kicks() || pos == Position::LS;
    let man = man_share(w, p.college);
    let two_gap = pos == Position::DT
        && p.college.is_some_and(|c| w.programs[c].installed.front() == crate::game::roster::Front::ThreeFour);
    let (mut comp, mut cw) = (0.0, 0.0);
    let (mut gsum, mut gn) = (0.0, 0.0);
    let last = recs.last().map_or(0, |r| r.year);
    for r in &recs {
        let snaps = r.get(Stat::SnapsOff) + r.get(Stat::SnapsDef) + if kicks { r.get(Stat::SnapsSt) } else { 0.0 };
        f.snaps += snaps;
        f.role[PASS_PRO] += r.get(Stat::PbSnaps);
        f.role[RUN_BLOCK] += r.get(Stat::RbSnaps);
        // A two-gap tackle reads and holds his gaps: few of his snaps are
        // a real rush.
        f.role[PASS_RUSH] += r.get(Stat::RushSnaps) * if two_gap { 0.4 } else { 1.0 };
        f.role[MAN] += r.get(Stat::CovSnaps) * man;
        f.role[ZONE] += r.get(Stat::CovSnaps) * (1.0 - man);
        f.role[ROUTES] += r.get(Stat::Routes);
        f.role[DROPBACKS] += r.get(Stat::Dropbacks);
        f.role[CARRIES] += r.get(Stat::Rush);
        f.role[KICKS] += r.get(Stat::FgAtt) + r.get(Stat::Punts) + r.get(Stat::XpMade);
        comp += snaps * (r.sos as f64 + div_offset(r.division));
        cw += snaps;
        if r.year + 1 >= last {
            gsum += r.get(Stat::GradeSum);
            gn += r.get(Stat::Graded);
        }
    }
    f.known = (1 << ROLES) - 1;
    f.starts = f.snaps / 700.0;
    f.comp = if cw > 0.0 { comp / cw } else { f.division.map_or(0.0, div_offset) };
    if gn >= 50.0 {
        let avg = gsum / gn;
        let conf = (gn / (gn + 25.0)).sqrt();
        let g = clamp(60.0 + 95.0 * avg * conf, 20.0, 99.9);
        f.grade = Some(g);
        f.dom = clamp((g - 70.0) / 15.0, 0.0, 1.5);
    }
    f.context = context(w, i, pos);
}

/// SK4: how his system and the men around him lift his numbers. A
/// pass-happy system lifts a passer's and his receivers' yards; a great
/// line lifts a quarterback's and a back's; a great quarterback lifts his
/// receivers'. Teammates are read against his conference.
fn context(w: &World, i: usize, pos: Position) -> f64 {
    use crate::game::roster::OffScheme as O;
    let p = &w.people[i];
    let Some(c) = p.college else { return 0.0 };
    let scheme = match (w.programs[c].installed.offense(), pos) {
        (O::AirRaid, Position::QB) => 0.9,
        (O::AirRaid, Position::WR) => 0.7,
        (O::AirRaid, Position::RB) => -0.3,
        (O::WestCoast, Position::QB | Position::WR | Position::TE) => 0.3,
        (O::SpreadOption, Position::QB) => 0.3,
        (O::SpreadOption | O::WideZone | O::PowerGap, Position::RB) => 0.4,
        (O::PowerGap, Position::QB | Position::WR) => -0.4,
        (O::WideZone, Position::QB) => -0.1,
        _ => 0.0,
    };
    let unit = match pos {
        Position::QB | Position::RB => Some((crate::world::coach::Group::OL, 5)),
        Position::WR | Position::TE => Some((crate::world::coach::Group::QB, 1)),
        _ => None,
    };
    let Some((g, n)) = unit else { return clamp(scheme, -2.0, 2.0) };
    let rel = unit_rel(w, c, g, n);
    clamp(scheme + rel, -2.0, 2.0)
}

/// The day's stamp, and each (program, group, size)'s unit against its
/// conference.
type UnitCache = (u64, HashMap<(usize, usize, usize), f64>);

thread_local! {
    /// Each program's unit against its conference, kept for the day (the
    /// rosters don't move within it): every read of every player at the
    /// program asks the same question.
    static UNITS: std::cell::RefCell<UnitCache> = Default::default();
}

/// His unit (the line, the quarterback, the receivers) against the rest of
/// his conference's, in fifths of a rating point's spread.
fn unit_rel(w: &World, c: usize, g: crate::world::coach::Group, n: usize) -> f64 {
    let stamp = (w.today.0 as u64) << 32 ^ (w.options.seed as u64) << 8 ^ (w.people.len() as u64).rotate_left(44);
    let key = (c, g as usize, n);
    if let Some(v) = UNITS.with(|u| {
        let mut u = u.borrow_mut();
        if u.0 != stamp {
            u.0 = stamp;
            u.1.clear();
        }
        u.1.get(&key).copied()
    }) {
        return v;
    }
    let mine = unit_mean(w, c, g, n);
    let prog = &w.programs[c];
    let (mut s, mut k) = (0.0, 0.0);
    for (q, other) in w.programs.iter().enumerate() {
        if q != c
            && other.conference == prog.conference
            && other.division == prog.division
            && other.plays(w.year)
            && let Some(m) = unit_mean(w, q, g, n)
        {
            s += m;
            k += 1.0;
        }
    }
    let rel = match (mine, k > 0.0) {
        (Some(m), true) => (m - s / k) / 5.0,
        _ => 0.0,
    };
    UNITS.with(|u| u.borrow_mut().1.insert(key, rel));
    rel
}

/// The mean overall of a program's best `n` at a coaching group.
fn unit_mean(w: &World, prog: usize, g: crate::world::coach::Group, n: usize) -> Option<f64> {
    let mut v: Vec<f64> = w.programs[prog]
        .roster
        .iter()
        .filter_map(|&k| w.people.get(k))
        .filter(|q| crate::world::coach::Group::of(q.player.position) == g)
        .map(|q| q.player.overall())
        .collect();
    if v.len() < n {
        return None;
    }
    v.sort_by(|a, b| b.total_cmp(a));
    Some(v[..n].iter().sum::<f64>() / n as f64)
}

fn high_school(w: &World, i: usize, pos: Position, f: &mut Footage) {
    use crate::world::hs_year::HsStat as H;
    let p = &w.people[i];
    f.level = 1;
    f.reels = w.year >= 2008;
    let hs = w.high_schools.get(p.hs);
    let culture = w.states.get(p.home_state).map_or(0.0, |s| s.culture);
    f.comp = -1.5 * hs.map_or(2.0, |h| h.classification as f64) + 3.0 * culture - 3.0;
    if p.hs_lines.is_empty() {
        return;
    }
    let (mut games, mut prod) = (0.0, 0.0);
    for l in &p.hs_lines {
        f.snaps += l.get(H::SnapsOff) + l.get(H::SnapsDef);
        f.role[DROPBACKS] += l.get(H::Att);
        f.role[CARRIES] += l.get(H::Rush);
        f.role[ROUTES] += 6.0 * l.get(H::Rec);
        f.role[KICKS] += l.get(H::FgAtt) + l.get(H::XpMade);
        games += l.games as f64;
        prod += match pos {
            Position::QB => (l.get(H::PassYds) + l.get(H::RushYds)) / 300.0,
            Position::RB => (l.get(H::RushYds) + l.get(H::RecYds)) / 150.0,
            Position::WR | Position::TE => l.get(H::RecYds) / 100.0,
            Position::EDGE | Position::DT => (l.get(H::Sacks) + l.get(H::Tfl)) / 1.6,
            Position::LB => l.get(H::Tackles) / 10.0,
            Position::CB | Position::S => (l.get(H::Ints) + l.get(H::Pbus)) / 1.0,
            _ => 0.0,
        };
    }
    f.known = 1 << DROPBACKS | 1 << CARRIES | 1 << ROUTES | 1 << KICKS;
    f.starts = f.snaps / 450.0;
    if games > 0.0 {
        f.dom = clamp((prod / games - 0.7) / 0.5, 0.0, 1.5);
    }
}

/// SK1: how far a film look can narrow a read (a multiple of its misread).
/// No starts stays wide (1.55), one season 1.28, two 1.01, three or more
/// 0.75. With no record kept, a typical look (1.0).
pub fn film_factor(f: &Footage) -> f64 {
    if f.level == 0 || (f.snaps == 0.0 && f.level == 1) {
        return 1.0;
    }
    clamp(1.55 - 0.27 * f.starts, 0.75, 1.55)
}

/// The role a skill needs his snaps in, if any.
pub fn role_of(a: Attr, pos: Position) -> Option<usize> {
    use Attr::*;
    Some(match a {
        PassBlocking | Anchor => PASS_PRO,
        RunBlocking | BlockingInSpace if !matches!(pos, Position::WR | Position::RB) => RUN_BLOCK,
        PowerRush | FinesseRush => PASS_RUSH,
        Blitzing => PASS_RUSH,
        ManCoverage | Press => MAN,
        ZoneCoverage => ZONE,
        RouteRunning | Release | Catching | ContestedCatch | BallTracking => ROUTES,
        ArmStrength | ShortAccuracy | MediumAccuracy | DeepAccuracy | Touch | ThrowOnMove | QuickRelease
        | PocketPresence | EyeManipulation => DROPBACKS,
        BallCarrierVision | Elusiveness | LegDrive => CARRIES,
        a if a.category() == Category::KickingAndSnapping && a != LongSnapping && a != ShotgunSnap => KICKS,
        _ => return None,
    })
}

/// SK3: whether his tape shows a skill (a skill his role rarely tested
/// stays unseen, not average). A record that doesn't count the role reads
/// as shown.
pub fn role_seen(f: &Footage, a: Attr, pos: Position) -> bool {
    match role_of(a, pos) {
        Some(r) if f.known & (1 << r) != 0 => f.role[r] >= ROLE_MIN,
        _ => true,
    }
}

/// How much a reader leans on numbers over the tape (SK4): a weaker judge
/// leans more.
pub fn numbers_lean(judge: f64) -> f64 {
    clamp((14.0 - judge) / 8.0, 0.0, 1.0) * 0.6
}

/// The shared tilt of a scout's or coach's read, in rating points, on a
/// skill (SK2, SK4, SK5): dominance over weak opponents until he has been
/// seen against good players, his system and teammates to a reader who
/// leans on numbers, a reel's best snaps, and part of his last season's
/// form (the tape shows the same plays).
pub fn scout_tilt(f: &Footage, judge: f64, seen_vs_good: bool, reel_only: bool) -> f64 {
    let weak = clamp(-f.comp / 6.0, 0.0, 1.5);
    let comp = 2.2 * f.dom * weak * if seen_vs_good { 0.4 } else { 1.0 };
    let ctx = numbers_lean(judge) * 2.5 * f.context;
    let reel = if reel_only && f.reels { 2.5 } else { 0.0 };
    comp + ctx + reel + 0.3 * f.luck
}

/// SK6: the analytics staff reads his numbers: production against his
/// level (so weak opponents don't fool it), his age, his testing. Where
/// the numbers mislead (a hot or cold year, part of a flattering system),
/// it misleads with them.
pub fn analytics_tilt(f: &Footage) -> f64 {
    0.8 * f.luck + 0.75 * f.context
}

/// How well the numbers read a skill: base misread 8 at a starter's three
/// seasons, wider with fewer snaps.
pub fn analytics_sd(f: &Footage) -> f64 {
    8.0 * clamp((2100.0 / f.snaps.max(100.0)).sqrt(), 0.8, 3.0)
}

/// Whether the numbers say anything about a skill: a role skill once his
/// role has the snaps, and the film mind (production shows decisions and
/// motor); not his talk or his body.
pub fn numbers_see(f: &Footage, a: Attr, pos: Position) -> bool {
    if f.level != 2 || f.snaps < 100.0 {
        return false;
    }
    match a.category() {
        Category::Physical => false,
        Category::Mental => matches!(a, Attr::Decisions | Attr::Motor | Attr::Anticipation | Attr::PlayRecognition),
        _ => role_of(a, pos).is_some_and(|r| f.role[r] >= ROLE_MIN),
    }
}

/// The plain reasons a file shows for how its reads came out (never a
/// true number): what the tape holds and how it leans.
pub fn reasons(f: &Footage, pos: Position, seen_vs_good: bool, reel_only: bool) -> Vec<String> {
    let mut v = Vec::new();
    match f.level {
        2 => {
            let starts = f.starts;
            if f.snaps < 250.0 {
                v.push("Barely on the field: film can't narrow him much.".to_string());
            } else if starts < 1.5 {
                v.push(format!(
                    "About {} of starter snaps on tape: his ranges stay wide.",
                    if starts < 1.0 { "half a season" } else { "one season" }
                ));
            } else if starts >= 2.8 {
                v.push("Three seasons of starts on tape: film narrows him.".to_string());
            }
        }
        1 if f.snaps > 0.0 => {
            if f.starts < 1.0 {
                v.push("Less than a varsity season on tape.".to_string());
            } else if f.starts >= 2.8 {
                v.push("Three varsity seasons on tape.".to_string());
            }
        }
        _ => {}
    }
    let weak = clamp(-f.comp / 6.0, 0.0, 1.5);
    if f.dom > 0.4 && weak > 0.4 {
        v.push(if seen_vs_good {
            "Dominated weak opponents; seen against good players since.".to_string()
        } else {
            "Dominated weak opponents and hasn't been seen against good players.".to_string()
        });
    }
    let unseen: Vec<&str> = [
        (PASS_PRO, "pass sets"),
        (RUN_BLOCK, "run blocking"),
        (PASS_RUSH, "pass rush"),
        (MAN, "press and man coverage"),
        (ZONE, "zone coverage"),
        (ROUTES, "routes and hands"),
        (DROPBACKS, "throwing"),
        (CARRIES, "running the ball"),
    ]
    .iter()
    .filter(|(r, _)| f.known & (1 << r) != 0 && f.role[*r] < ROLE_MIN && wants(pos, *r))
    .map(|x| x.1)
    .collect();
    if !unseen.is_empty() {
        v.push(format!("His role rarely tested his {}: not seen on tape.", unseen.join(", ")));
    }
    if f.context >= 0.6 {
        v.push("His numbers come in a system and with teammates that lift them.".to_string());
    } else if f.context <= -0.6 {
        v.push("His system and teammates hold his numbers down.".to_string());
    }
    if reel_only && f.reels {
        v.push("Seen on his own highlight reel only: it shows his best snaps.".to_string());
    }
    v
}

/// The roles a position's grade leans on (what a scout would miss).
fn wants(pos: Position, r: usize) -> bool {
    use Position::*;
    match r {
        PASS_PRO | RUN_BLOCK => matches!(pos, OT | IOL),
        PASS_RUSH => matches!(pos, EDGE | DT),
        MAN | ZONE => matches!(pos, CB | S),
        ROUTES => matches!(pos, WR | TE),
        DROPBACKS => pos == QB,
        CARRIES => pos == RB,
        _ => false,
    }
}

/// What a staff's system asks of a player at his spot (bug 18's fit):
/// the two or three skills the installed scheme, front or shell leans on.
/// Kickers, punters and snappers have none.
pub fn scheme_asks(inst: &crate::world::systems::Installed, pos: Position) -> &'static [Attr] {
    use crate::game::roster::{Front, OffScheme as O, Shell};
    use Attr::*;
    use Position as P;
    match pos {
        P::QB => match inst.offense() {
            O::AirRaid => &[QuickRelease, ShortAccuracy, MediumAccuracy],
            O::WestCoast => &[ShortAccuracy, Touch, QuickRelease],
            O::WideZone => &[ThrowOnMove, Touch, BallHandling],
            O::PowerGap => &[ArmStrength, PocketPresence, DeepAccuracy],
            O::SpreadOption => &[Speed, BallHandling, ThrowOnMove],
            O::WingT => &[BallHandling, Touch],
        },
        P::RB => match inst.offense() {
            O::WideZone => &[BallCarrierVision, Acceleration],
            O::PowerGap | O::WingT => &[LegDrive, BallSecurity],
            O::SpreadOption => &[Speed, Elusiveness],
            O::AirRaid | O::WestCoast => &[Catching, RouteRunning, PassBlocking],
        },
        P::WR => match inst.offense() {
            O::AirRaid => &[RouteRunning, Release, Speed],
            O::WestCoast => &[RouteRunning, Catching],
            O::WideZone | O::PowerGap | O::WingT => &[BlockingInSpace, ContestedCatch],
            O::SpreadOption => &[Speed, BlockingInSpace],
        },
        P::TE => match inst.offense() {
            O::WideZone | O::PowerGap | O::WingT => &[RunBlocking, BlockingInSpace],
            _ => &[RouteRunning, Catching],
        },
        P::OT | P::IOL => match inst.offense() {
            O::WideZone => &[Agility, BlockingInSpace, Acceleration],
            O::PowerGap => &[Strength, RunBlocking, Anchor],
            O::AirRaid | O::WestCoast => &[PassBlocking, Anchor, HandUse],
            O::SpreadOption | O::WingT => &[Agility, RunBlocking],
        },
        P::EDGE | P::DT => match inst.front() {
            Front::FourThree => &[FinesseRush, Acceleration, PowerRush],
            Front::ThreeFour => &[Strength, Anchor, BlockShedding],
        },
        P::LB => match inst.front() {
            Front::FourThree => &[Pursuit, Speed, ZoneCoverage],
            Front::ThreeFour => &[Blitzing, PowerRush, BlockShedding],
        },
        P::CB => match inst.shell() {
            Shell::ManPressure => &[ManCoverage, Press, Speed],
            Shell::Cover3 => &[ZoneCoverage, Press],
            Shell::Quarters => &[ZoneCoverage, ManCoverage],
            Shell::Cover2 => &[ZoneCoverage, Tackling],
        },
        P::S => match inst.shell() {
            Shell::ManPressure => &[ManCoverage, Speed],
            Shell::Cover3 => &[Pursuit, Tackling, Speed],
            Shell::Quarters => &[ZoneCoverage, Tackling],
            Shell::Cover2 => &[ZoneCoverage, BallTracking],
        },
        P::K | P::P | P::LS => &[],
    }
}
