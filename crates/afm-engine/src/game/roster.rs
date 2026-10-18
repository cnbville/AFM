//! Who plays: game-day players, the coaching staff, schemes, the depth chart,
//! two-way players (topic 5.1), fatigue and rotations (topic 6).

use crate::attributes::{Attr, Attributes};
use crate::body::body_now;
use crate::player::{GameTraits, Personality, Player};
use crate::positions::{POSITION_COUNT, Position, overall_for};
use crate::rng::clamp;
use crate::spots::{self, SPOT_COUNT, Spot, fill_in, spot_rating};

use super::calling::Film;
use super::playbook::FAMILY_COUNT;

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
    /// What he knows of each concept family, 0-1 (plays and tactics topic
    /// 4): a third is knowing his job, two thirds understanding why, all of
    /// it mastery.
    pub know: [f32; FAMILY_COUNT],
    /// Seasons of real game experience.
    pub experience: f64,
    /// How much his coach lets him freelance, 0-1 (topic 3.1). Moves with
    /// what happens when he does.
    pub trust: f64,
    /// His listed spot, the spots he has been listed at (bits) and his
    /// rating at every spot (fix list topic 3).
    pub spot: Spot,
    pub spots_played: u32,
    pub spot_r: [f64; SPOT_COUNT],
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
            know: [clamp(knowledge, 0.0, 1.0) as f32; FAMILY_COUNT],
            experience,
            trust: 0.5,
            spot: p.spot,
            spots_played: p.spots_played | p.spot.bit(),
            spot_r: Spot::ALL.map(|s| spot_rating(s, &p.attributes, b.weight)),
        }
    }

    /// Rating points he gives up at a position he isn't listed at: the
    /// least he'd lose at one of its spots (3.3). Nothing at a spot he has
    /// played.
    pub fn cross_cost(&self, pos: Position) -> f64 {
        spots::spots_of(pos)
            .iter()
            .filter(|&&s| s != Spot::FB)
            .map(|&s| if self.spots_played & s.bit() != 0 { 0.0 } else { fill_in(s, self.spot) })
            .fold(f64::INFINITY, f64::min)
    }

    #[inline]
    pub fn a(&self, a: Attr) -> f64 {
        self.attrs[a]
    }

    pub fn overall_at(&self, pos: Position) -> f64 {
        overall_for(pos, &self.attrs)
    }

    /// An attribute on the 1-20 scale the engine's hidden traits used
    /// (five rating points a step).
    #[inline]
    pub fn on20(&self, a: Attr) -> f64 {
        clamp(self.a(a) / 5.0, 1.0, 20.0)
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

    /// Acceleration, yards per second squared: his grip for braking and
    /// cutting. The burst from a standing start is `sprint_tau`.
    pub fn accel(&self) -> f64 {
        // Fit with `drills` so the sim's 5-10-5 shuttle and 3-cone move per
        // point as the combine model does (`body::combine`), the way the 40
        // was fit (dynasty item 1; was 0.035 a point, which made a quick
        // player 3-8 times quicker in the sim than at the combine). Below the
        // pro range (most high school players) the gap narrows (proposed,
        // not confirmed).
        let ac = self.a(Attr::Acceleration);
        let ac = if ac >= 60.0 { 4.65 + 0.008 * (ac - 60.0) } else { 4.65 + 0.0046 * (ac - 60.0) };
        clamp(ac - 0.006 * (self.weight - 220.0), 2.2, 7.5)
    }

    /// How hard he can cut, as a share of his acceleration spent sideways.
    pub fn cut(&self) -> f64 {
        // As with acceleration, the gap narrows below the pro range, so a
        // high school athlete still cuts like an athlete.
        let ag = self.a(Attr::Agility);
        // Fit with `drills` as above (was 0.011 a point).
        let ag = if ag >= 60.0 { 0.99 + 0.004 * (ag - 60.0) } else { 0.99 + 0.0022 * (ag - 60.0) };
        clamp(ag - 0.002 * (self.weight - 220.0), 0.4, 1.7)
    }

    /// Composure, 1-20.
    pub fn composure(&self) -> f64 {
        self.on20(Attr::Composure)
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
    /// Play-calling quality, 1-20 (hidden ability): the mean of his skills.
    pub skill: f64,
    /// How fast and how well he adjusts during a game, 1-20.
    pub adaptation: f64,
    /// 1-20: deep shots, blitzes, fourth downs. A style, not a quality.
    pub aggressiveness: f64,
    /// Plays and tactics topic 6.2, 1-20 each: reading what the other side
    /// shows and what beats it; situational knowledge (down, distance,
    /// field zone); creativity (calls that fit his players, breaking his
    /// own tendencies).
    pub reading: f64,
    pub situational: f64,
    pub creativity: f64,
    /// Under pressure (behind late, a job on the line), -1 tightens up, +1
    /// gambles (topic 6.5), from his personality.
    pub nerve: f64,
}

impl Caller {
    pub fn average() -> Caller {
        Caller::new(10.0, 10.0, 10.0)
    }
    /// A caller whose skills all sit at his overall quality.
    pub fn new(skill: f64, adaptation: f64, aggressiveness: f64) -> Caller {
        Caller { skill, adaptation, aggressiveness, reading: skill, situational: skill, creativity: skill, nerve: 0.0 }
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
    /// The defensive coordinator's personnel habit (S8): how often he meets
    /// three receivers with nickel instead of staying in base. Real NFL
    /// teams answered 11 personnel with nickel 88% of the time.
    pub nickel: f64,
    /// The season: what the era plays (RPOs, motion, the triple option).
    pub year: i32,
    /// The staff's film study, 1-20 (topic 5.1): coordinators plus quality
    /// control. How much of the opponent's tendencies it finds.
    pub film_study: f64,
    /// The install (calendar T6.2, plays and tactics topic 4.6): the share
    /// of the playbook in a normal week's game plan, 0.35-1. A bigger plan
    /// carries more calls and more mental errors for players who can't
    /// absorb it.
    pub install: f64,
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
            install: INSTALL,
            nickel: 0.88,
            year: 2026,
            film_study: 10.0,
        }
    }
}

/// A team for one game.
#[derive(Clone, Debug)]
pub struct GameTeam {
    pub name: String,
    pub players: Vec<GamePlayer>,
    pub staff: Staff,
    /// What the team has shown this season (topic 5.1): the other staff
    /// studies it.
    pub film: Film,
    /// How long the offensive line has played together, 0-1 (topic 4.9):
    /// cleaner calls, stunts passed off.
    pub line_chem: f64,
    /// How new each concept family still is to defenses at this level, 0-1
    /// (topic 9): a fresh idea is read a beat late.
    pub novelty: [f64; FAMILY_COUNT],
    /// This week's preparation time against a normal week (calendar T6.2):
    /// a short week (a Thursday game after a Saturday) about 0.8, after a
    /// bye about 1.1.
    pub week: f64,
}

/// A normal week's install, as a share of the playbook (proposed, not
/// confirmed).
pub const INSTALL: f64 = 0.75;

impl GameTeam {
    /// A team with no film, a settled line, nothing new and a normal week.
    pub fn new(name: String, players: Vec<GamePlayer>, staff: Staff) -> GameTeam {
        GameTeam {
            name,
            players,
            staff,
            film: Film::default(),
            line_chem: 0.6,
            novelty: [0.0; FAMILY_COUNT],
            week: 1.0,
        }
    }

    /// The plan the staff takes into this game and the load it puts on the
    /// players (T6.2). A short week cuts the plan, but only half the
    /// shortfall: real staffs trim the install and still see more mental
    /// errors. After a bye they add a little. Returns (the share of the
    /// playbook called, the load against a normal week's learning).
    pub fn plan(&self) -> (f64, f64) {
        let w = self.week.max(0.3);
        let planned = (self.staff.install * (1.0 - 0.5 * (1.0 - w))).clamp(0.2, 1.0);
        (planned, planned / w)
    }
}

/// How much of a game plan a player can absorb in a normal week, by his
/// awareness (T6.2): about all of a full plan at 75, two thirds at 60,
/// under half at 50 (proposed, not confirmed).
pub fn absorb(awareness: f64) -> f64 {
    ((awareness - 30.0) / 45.0).clamp(0.2, 1.3)
}

/// Days since a team's last game to its preparation (T6.2): four days or
/// fewer is a very short week, five a Thursday game, a week normal, a bye
/// more (proposed, not confirmed).
pub fn prep_week(days: Option<i32>) -> f64 {
    match days {
        None => 1.15,
        Some(d) if d <= 4 => 0.65,
        Some(5) => 0.8,
        Some(6) => 0.92,
        Some(d) if d <= 9 => 1.0,
        Some(d) if d <= 13 => 1.1,
        Some(_) => 1.15,
    }
}

/// The depth chart: players (indexes into `players`) in order at each spot.
#[derive(Clone, Debug, Default)]
pub struct Depth {
    pub at: [Vec<usize>; POSITION_COUNT],
    /// Kickoff kickers, best leg first (fix list 1.8). With three or four
    /// kickers the strongest leg kicks off and the field goal kicker
    /// (accuracy and leg, the K list) kicks field goals.
    pub kickoff: Vec<usize>,
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
        // A backup at each specialist spot, so an injury falls to the next leg.
        Position::K | Position::P | Position::LS => 2,
    }
}

fn offense(pos: Position) -> bool {
    matches!(pos, Position::QB | Position::RB | Position::WR | Position::TE | Position::OT | Position::IOL)
}

/// Rating points a two-way corner or receiver must beat the starters by
/// on the other side (3.5): good enough at both, after what playing a spot
/// he doesn't practice costs him.
pub const CROSS_LIST: f64 = 3.0;

/// The head coach's depth chart (topic 5.1). Each spot lists the players
/// at that position first, best first. A player from another position in
/// the same group (fix list 3.1-3.2: tackles and guards, ends and tackles,
/// corners and safeties) is listed there only when he beats the players he
/// would pass after what the unfamiliar spot costs him (3.3), and never
/// ahead of others if he starts at his own spot. Another group's player is
/// listed only when the group runs out of bodies for its starters, an
/// emergency (3.6). A player is added to the other side's chart when he
/// would beat the starters there by `two_way_margin` rating points: small
/// high schools play their best athletes both ways (3.10); college and the
/// NFL never do (99), except a corner who is good enough at receiver, or a
/// receiver at corner (3.5, 3.11).
///
/// The kicking jobs go to the kickers and punters a team carries: with only
/// one of them, he does both (fix list 1.9), and only a team with neither
/// asks the best leg on the roster. A listed long snapper snaps; he plays
/// nowhere else (fix list 1.7).
pub fn depth_chart(team: &GameTeam, two_way_margin: f64) -> Depth {
    let ps = &team.players;
    let mut d = Depth::default();
    let has_legs = ps.iter().any(|p| p.position.kicks());
    let has_snapper = ps.iter().any(|p| p.position == Position::LS);
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
        let same_side = |p: &GamePlayer| match pos {
            // A kicker kicks and a punter punts: the other leg is only the
            // backup there; a team with neither asks the best leg it has.
            Position::K | Position::P => !has_legs || p.position.kicks(),
            // A listed long snapper snaps; without one, anyone can.
            Position::LS => !has_snapper || p.position == Position::LS,
            _ => !p.position.is_specialist() && spots::group(p.position) == spots::group(pos),
        };
        let kicks = pos.kicks() && has_legs;
        let mut cross: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(i, p)| p.position != pos && same_side(p) && !starting[*i])
            .map(|(i, p)| (p.overall_at(pos) - if kicks { 100.0 } else { p.cross_cost(pos) }, i))
            .collect();
        cross.sort_by(|a, b| b.0.total_cmp(&a.0));
        list.extend(cross);
        list.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = wanted(pos).max(n_own);
        list.truncate(want);
        // Cover inside the group: anyone left in it, starters elsewhere last.
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
        // An emergency (3.6): the group can't field its starters, so the
        // closest fit on this side of the ball from another group, rated at
        // his true fit there, starters elsewhere last.
        if list.len() < starters(pos) && !pos.is_specialist() {
            let mut more: Vec<(f64, usize)> = ps
                .iter()
                .enumerate()
                .filter(|(i, p)| {
                    !p.position.is_specialist()
                        && offense(p.position) == offense(pos)
                        && !list.iter().any(|l| l.1 == *i)
                })
                .map(|(i, p)| (p.overall_at(pos) - if starting[i] { 50.0 } else { 0.0 }, i))
                .collect();
            more.sort_by(|a, b| b.0.total_cmp(&a.0));
            list.extend(more.into_iter().take(starters(pos) - list.len()));
        }
        d.at[pos.index()] = list.into_iter().map(|c| c.1).collect();
    }
    // Two-way players: the other side's best athletes can start here too.
    for pos in Position::ALL.into_iter().filter(|p| !p.is_specialist()) {
        let list = &d.at[pos.index()];
        let worst_starter =
            list.iter().take(starters(pos)).map(|&i| ps[i].overall_at(pos)).fold(f64::INFINITY, f64::min);
        // The corner and receiver exception (3.5) holds at every level.
        let swap = |p: &GamePlayer| {
            p.position == Position::CB && pos == Position::WR || p.position == Position::WR && pos == Position::CB
        };
        let mut extra: Vec<(f64, usize)> = ps
            .iter()
            .enumerate()
            .filter(|(_, p)| offense(p.position) != offense(pos) && !p.position.is_specialist())
            .map(|(i, p)| (p.overall_at(pos), i, swap(p)))
            .filter(|&(r, _, sw)| {
                let margin = if sw { two_way_margin.min(CROSS_LIST) } else { two_way_margin };
                r > worst_starter + margin || list.len() < starters(pos)
            })
            .map(|(r, i, _)| (r, i))
            .collect();
        if extra.is_empty() {
            continue;
        }
        extra.sort_by(|a, b| b.0.total_cmp(&a.0));
        let want = list.len().max(wanted(pos));
        let mut merged: Vec<(f64, usize)> = list
            .iter()
            .map(|&i| (ps[i].overall_at(pos) - if ps[i].position == pos { 0.0 } else { ps[i].cross_cost(pos) }, i))
            .chain(extra.into_iter().take(starters(pos)))
            .collect();
        merged.sort_by(|a, b| b.0.total_cmp(&a.0));
        merged.dedup_by_key(|m| m.1);
        d.at[pos.index()] = merged.into_iter().take(want).map(|m| m.1).collect();
    }
    let best = |f: &dyn Fn(&GamePlayer) -> f64, skip: &[usize]| -> usize {
        (0..ps.len()).filter(|i| !skip.contains(i)).max_by(|&a, &b| f(&ps[a]).total_cmp(&f(&ps[b]))).unwrap_or(0)
    };
    // The kicking legs (fix list 1.8, 1.9). The K and P lists already rank
    // every leg at each job, the listed specialist first: a team with only
    // a kicker or only a punter has him do both, rated at each from the
    // same leg. The kickoff goes to the best leg among the kickers.
    let legs = |pos: Position| -> Vec<usize> { (0..ps.len()).filter(|&i| ps[i].position == pos).collect() };
    let mut kickoff = legs(Position::K);
    if kickoff.is_empty() {
        kickoff = legs(Position::P);
    }
    if kickoff.is_empty() {
        kickoff = d.list(Position::K).to_vec();
    }
    kickoff.sort_by(|&a, &b| ps[b].a(Attr::KickPower).total_cmp(&ps[a].a(Attr::KickPower)));
    d.kickoff = kickoff;
    // The long snapper (fix list 1.7): the listed snapper, or else the best
    // snapper among players not already starting at the spots that take
    // the field together with him.
    let busy: Vec<usize> =
        [Position::QB, Position::K, Position::P].iter().flat_map(|p| d.list(*p).iter().take(1).copied()).collect();
    d.snapper = d
        .list(Position::LS)
        .iter()
        .copied()
        .find(|&i| ps[i].position == Position::LS && !busy.contains(&i))
        .unwrap_or_else(|| best(&|p| p.a(Attr::LongSnapping) + 0.1 * p.a(Attr::FootballIq), &busy));
    // The holder: the punter, unless he is the one kicking; else the backup quarterback.
    let kicker = d.list(Position::K).first().copied();
    d.holder = d
        .list(Position::P)
        .iter()
        .copied()
        .find(|&i| ps[i].position == Position::P && Some(i) != kicker && i != d.snapper)
        .unwrap_or_else(|| d.list(Position::QB)[1.min(d.list(Position::QB).len() - 1)]);
    let returner = |p: &GamePlayer| {
        0.4 * p.a(Attr::Speed)
            + 0.25 * p.a(Attr::Elusiveness)
            + 0.2 * p.a(Attr::BallCarrierVision)
            + 0.15 * p.a(Attr::BallTracking)
    };
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
        Position::QB | Position::RB | Position::TE | Position::K | Position::P | Position::LS => 1,
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
