//! Who plays: game-day players, the coaching staff, schemes, the depth chart,
//! two-way players (topic 5.1), fatigue and rotations (topic 6).

use crate::attributes::{Attr, Attributes};
use crate::body::body_now;
use crate::player::{GameTraits, Personality, Player};
use crate::positions::{Position, overall_for};
use crate::rng::clamp;

/// A player as the match engine sees him on game day.
#[derive(Clone, Debug)]
pub struct GamePlayer {
    pub id: u32,
    /// Listed position.
    pub position: Position,
    pub age: u32,
    pub attrs: Attributes,
    /// Inches and pounds today.
    pub height: f64,
    pub weight: f64,
    pub arm: f64,
    pub traits: GameTraits,
    pub personality: Personality,
    pub injury_proneness: f64,
    /// How well he knows this team's playbook, 0-1 (topic 3).
    pub knowledge: f64,
    /// Seasons of real game experience.
    pub experience: f64,
    /// How much his coach lets him freelance, 0-1 (topic 3.1). Moves with
    /// what happens when he does.
    pub trust: f64,
}

impl GamePlayer {
    /// `knowledge` is his grasp of this team's playbook (0-1) and
    /// `experience` his seasons of real games; the caller knows both.
    pub fn from_player(p: &Player, knowledge: f64, experience: f64) -> GamePlayer {
        let b = body_now(p);
        GamePlayer {
            id: p.id,
            position: p.position,
            age: p.age,
            attrs: p.attributes,
            height: b.height,
            weight: b.weight,
            arm: b.arm,
            traits: p.game,
            personality: p.personality,
            injury_proneness: p.injury_proneness,
            knowledge: clamp(knowledge, 0.0, 1.0),
            experience,
            trust: 0.5,
        }
    }

    #[inline]
    pub fn a(&self, a: Attr) -> f64 {
        self.attrs[a]
    }

    pub fn overall_at(&self, pos: Position) -> f64 {
        overall_for(pos, &self.attrs)
    }

    /// His real 40 time from speed, burst and weight (same fit as the combine).
    pub fn forty(&self) -> f64 {
        5.56 - 0.0124 * (0.75 * self.a(Attr::Speed) + 0.25 * self.a(Attr::Acceleration))
            + 0.0033 * (self.weight - 220.0)
    }

    /// Top speed, yards per second. A 4.35 forty gives about 10.7 (21.9 mph),
    /// a 5.2 lineman about 9, in line with Next Gen Stats tracking. With the
    /// sprint build-up below, a player runs his own forty in the sim.
    pub fn top_speed(&self) -> f64 {
        40.0 / (self.forty() - self.sprint_tau() + 0.06).max(3.4)
    }

    /// How fast he builds to top speed: the time constant of his sprint, in
    /// seconds. Fit so the sim gives real 10-yard splits (about 0.345 of the
    /// forty: 1.5 for a 4.35 runner, 1.8 for a 5.2 lineman).
    pub fn sprint_tau(&self) -> f64 {
        clamp(0.78 + 0.003 * (60.0 - self.a(Attr::Acceleration)) + 0.0006 * (self.weight - 220.0), 0.55, 1.0)
    }

    /// Acceleration, yards per second squared.
    pub fn accel(&self) -> f64 {
        // Through the pro range each point counts fully; below it (most high
        // school players) the gap narrows (proposed, not confirmed).
        let ac = self.a(Attr::Acceleration);
        let ac = if ac >= 60.0 { 4.65 + 0.035 * (ac - 60.0) } else { 4.65 + 0.02 * (ac - 60.0) };
        clamp(ac - 0.006 * (self.weight - 220.0), 2.2, 7.5)
    }

    /// How hard he can cut, as a share of his acceleration spent sideways.
    pub fn cut(&self) -> f64 {
        // As with acceleration, the gap narrows below the pro range, so a
        // high school athlete still cuts like an athlete.
        let ag = self.a(Attr::Agility);
        let ag = if ag >= 60.0 { 0.99 + 0.011 * (ag - 60.0) } else { 0.99 + 0.006 * (ag - 60.0) };
        clamp(ag - 0.002 * (self.weight - 220.0), 0.4, 1.7)
    }

    /// Composure, 1-20: the hidden trait with today's poise.
    pub fn composure(&self) -> f64 {
        clamp(0.6 * self.traits.composure + 0.4 * self.a(Attr::Poise) / 5.0, 1.0, 20.0)
    }
}

/// Offensive systems (topic 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OffScheme {
    WestCoast,
    AirRaid,
    /// Shanahan-style wide zone with play action and boots.
    WideZone,
    /// Pro-style power and gap runs.
    PowerGap,
    /// Spread with zone read.
    SpreadOption,
    /// High school wing-T: misdirection runs, little passing.
    WingT,
}

impl OffScheme {
    pub const ALL: [OffScheme; 6] = [
        OffScheme::WestCoast,
        OffScheme::AirRaid,
        OffScheme::WideZone,
        OffScheme::PowerGap,
        OffScheme::SpreadOption,
        OffScheme::WingT,
    ];
    pub fn name(self) -> &'static str {
        ["West Coast", "Air Raid", "wide zone", "power gap", "spread option", "wing-T"][self as usize]
    }
    /// How much more (or less) than an average team this system passes.
    pub fn pass_lean(self) -> f64 {
        [0.02, 0.09, -0.03, -0.06, -0.02, -0.25][self as usize]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Front {
    FourThree,
    ThreeFour,
}

/// Coverage family a defensive coordinator leans on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    /// Single-high Cover 3 (Seattle).
    Cover3,
    /// Two-high quarters.
    Quarters,
    /// Man and pressure (Cover 1, Cover 0).
    ManPressure,
    /// Tampa 2 and Cover 2.
    Cover2,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caller {
    /// Play-calling quality, 1-20 (hidden ability).
    pub skill: f64,
    /// How fast and how well he adjusts during a game, 1-20.
    pub adaptation: f64,
    /// 1-20: deep shots, blitzes, fourth downs.
    pub aggressiveness: f64,
}

impl Caller {
    pub fn average() -> Caller {
        Caller { skill: 10.0, adaptation: 10.0, aggressiveness: 10.0 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Staff {
    pub offense: Caller,
    pub defense: Caller,
    /// Head coach clock and fourth-down judgment, 1-20 (topic 4.2).
    pub game_management: f64,
    /// Head coach strictness, 1-20 (topic 3.1). Visible: coaches build a
    /// reputation for it.
    pub strictness: f64,
    pub off_scheme: OffScheme,
    pub front: Front,
    pub shell: Shell,
    /// How many plays the team carries: high school small, NFL big (topic 3).
    pub playbook: f64,
    /// Head coach sportsmanship, 1-20 (hidden): how soon he takes his
    /// starters out of a decided game (dynasty item 3).
    pub sportsmanship: f64,
}

impl Staff {
    pub fn average(off_scheme: OffScheme) -> Staff {
        Staff {
            offense: Caller::average(),
            defense: Caller::average(),
            game_management: 10.0,
            strictness: 10.0,
            off_scheme,
            front: Front::FourThree,
            shell: Shell::Cover3,
            playbook: 1.0,
            sportsmanship: 10.0,
        }
    }
}

/// A team for one game.
#[derive(Clone, Debug)]
pub struct GameTeam {
    pub name: String,
    pub players: Vec<GamePlayer>,
    pub staff: Staff,
}

/// The depth chart: players (indexes into `players`) in order at each spot.
#[derive(Clone, Debug, Default)]
pub struct Depth {
    pub at: [Vec<usize>; 13],
    /// Long snapper, holder, kick and punt returners.
    pub snapper: usize,
    pub holder: usize,
    pub kick_returners: Vec<usize>,
    pub punt_returner: usize,
}

impl Depth {
    pub fn list(&self, pos: Position) -> &[usize] {
        &self.at[pos.index()]
    }
}

/// How many of each spot a team carries on game day.
fn wanted(pos: Position) -> usize {
    match pos {
        Position::QB => 2,
        Position::RB => 3,
        Position::WR => 6,
        Position::TE => 3,
        Position::OT => 3,
        Position::IOL => 4,
        Position::EDGE => 4,
        Position::DT => 4,
        Position::LB => 5,
        Position::CB => 5,
        Position::S => 3,
        Position::K | Position::P => 1,
    }
}

fn offense(pos: Position) -> bool {
    matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
}

/// Rating points a player gives up at a spot he isn't listed at: he knows
/// his own spot's technique and calls, not this one's. A coach moves him only
/// when he is better even after that.
pub const CROSS_LIST: f64 = 4.0;

/// The head coach's depth chart (topic 5.1). Each spot lists the players
/// at that position first, best first. A player from another spot on the
/// same side is listed there only when he beats the players he would pass by
/// `CROSS_LIST`, and never ahead of others if he starts at his own spot. On a
/// thin roster his best athletes also play the other side of the ball: a
/// player is added to the other side's chart when he would beat the players
/// there by `two_way_margin` rating points (0 for a small high school, large
/// for the NFL, where it never happens).
pub fn depth_chart(team: &GameTeam, two_way_margin: f64) -> Depth {
    let ps = &team.players;
    let mut d = Depth::default();
    let own = |pos: Position| -> Vec<(f64, usize)> {
        let mut v: Vec<(f64, usize)> =
            ps.iter().enumerate().filter(|(_, p)| p.position == pos).map(|(i, p)| (p.overall_at(pos), i)).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v
    };
    // Starters at their own spot stay there.
    let mut starting = vec![false; ps.len()];
    for pos in Position::ALL {
        for (_, i) in own(pos).into_iter().take(starters(pos)) {
            starting[i] = true;
        }
    }
    for pos in Position::ALL {
        let mut list = own(pos);
        let n_own = list.len();
        let same_side = |p: &GamePlayer| {
            if pos.is_specialist() || p.position.is_specialist() {
                pos.is_specialist() && p.position.is_specialist()
            } else {
                offense(p.position) == offense(pos)
            }
        };
        let mut cross: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(i, p)| p.position != pos && same_side(p) && !starting[*i])
            .map(|(i, p)| (p.overall_at(pos) - CROSS_LIST, i))
            .collect();
        cross.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.extend(cross);
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = wanted(pos).max(n_own);
        list.truncate(want);
        // Emergency cover: anyone left on this side, starters elsewhere last.
        if list.len() < want {
            let mut more: Vec<(f64, usize)> = ps
                .iter()
                .enumerate()
                .filter(|(i, p)| same_side(p) && !list.iter().any(|l| l.1 == *i))
                .map(|(i, p)| (p.overall_at(pos) - if starting[i] { 50.0 } else { 0.0 }, i))
                .collect();
            more.sort_by(|a, b| b.0.total_cmp(&a.0));
            list.extend(more.into_iter().take(want - list.len()));
        }
        d.at[pos.index()] = list.into_iter().map(|c| c.1).collect();
    }
    // Two-way players: the other side's best athletes can start here too.
    for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
        let list = &d.at[pos.index()];
        let worst_starter =
            list.iter().take(starters(pos)).map(|&i| ps[i].overall_at(pos)).fold(f64::INFINITY, f64::min);
        let mut extra: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(_, p)| offense(p.position) != offense(pos) && !p.position.is_specialist())
            .map(|(i, p)| (p.overall_at(pos), i))
            .filter(|&(r, _)| r > worst_starter + two_way_margin || list.len() < starters(pos))
            .collect();
        if extra.is_empty() {
            continue;
        }
        extra.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = list.len().max(wanted(pos));
        let mut merged: Vec<(f64, usize)> = list
            .iter()
            .map(|&i| (ps[i].overall_at(pos) - if ps[i].position == pos { 0.0 } else { CROSS_LIST }, i))
            .chain(extra.into_iter().take(starters(pos)))
            .collect();
        merged.sort_by(|a, b| b.0.total_cmp(&a.0));
        merged.dedup_by_key(|m| m.1);
        d.at[pos.index()] = merged.into_iter().take(want).map(|m| m.1).collect();
    }
    let best = |f: &dyn Fn(&GamePlayer) -> f64, skip: &[usize]| -> usize {
        (0..ps.len()).filter(|i| !skip.contains(i)).max_by(|&a, &b| f(&ps[a]).total_cmp(&f(&ps[b]))).unwrap_or(0)
    };
    // The long snapper: the best snapper among players not already starting
    // at the spots that take the field together with him.
    let busy: Vec<usize> =
        [Position::QB, Position::K, Position::P].iter().flat_map(|p| d.list(*p).iter().take(1).copied()).collect();
    d.snapper = best(&|p| p.traits.snapping + 0.02 * p.a(Attr::Awareness), &busy);
    d.holder = d
        .list(Position::P)
        .first()
        .copied()
        .unwrap_or_else(|| d.list(Position::QB)[1.min(d.list(Position::QB).len() - 1)]);
    let returner = |p: &GamePlayer| 0.5 * p.a(Attr::Speed) + 0.3 * p.a(Attr::Elusiveness) + 0.2 * p.a(Attr::Vision);
    let mut kr: Vec<usize> =
        d.list(Position::WR).iter().chain(d.list(Position::RB)).chain(d.list(Position::CB)).copied().collect();
    kr.sort_by(|&a, &b| returner(&ps[b]).total_cmp(&returner(&ps[a])));
    kr.dedup();
    // Starters rarely return kicks: a return is extra hits for a man the
    // offense or defense needs. Backups do it unless there are none
    // (proposed, not confirmed).
    let qb1 = d.list(Position::QB).first().copied();
    kr.retain(|&i| Some(i) != qb1);
    let starting: Vec<usize> = [Position::RB, Position::WR, Position::CB]
        .iter()
        .flat_map(|&p| d.list(p).iter().take(starters(p)).copied())
        .collect();
    let backups: Vec<usize> = kr.iter().copied().filter(|i| !starting.contains(i)).collect();
    if backups.len() >= 2 {
        kr = backups;
    }
    d.kick_returners = kr.iter().take(2).copied().collect();
    d.punt_returner = kr
        .iter()
        .copied()
        .max_by(|&a, &b| {
            let f = |p: &GamePlayer| returner(p) + 0.3 * p.a(Attr::Catching);
            f(&ps[a]).total_cmp(&f(&ps[b]))
        })
        .unwrap_or(0);
    d
}

/// Starters at each spot in a base set.
pub fn starters(pos: Position) -> usize {
    match pos {
        Position::QB | Position::RB | Position::TE | Position::K | Position::P => 1,
        Position::WR => 3,
        Position::OT => 2,
        Position::IOL => 3,
        Position::EDGE | Position::DT | Position::LB | Position::CB | Position::S => 2,
    }
}

/// Energy drained by one play, before his stamina (topic 6). Running and
/// fighting on every snap tires linemen and defenders most.
pub fn drain(stamina: f64, work: f64) -> f64 {
    work * (1.55 - 0.011 * stamina).max(0.4)
}

/// Energy regained between plays: a little on the field, more on the bench.
pub const RECOVER_ON_FIELD: f64 = 0.012;
pub const RECOVER_ON_BENCH: f64 = 0.05;
