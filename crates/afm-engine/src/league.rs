//! The pro league a class plays into: 32 teams with front offices, coaching
//! staffs and full depth charts. Only one high school class is simulated, so
//! everyone else on a roster is a background player: a rating and an age that
//! develop, age and leave like real veterans and are replaced by other classes'
//! draft picks and free agents. A class's players win and lose real depth chart
//! spots against them.

use crate::player::PlayingTime;
use crate::positions::{POSITION_COUNT, Position};
use crate::rng::{Rng, clamp, round_half_up};

pub const NFL_TEAM_NAMES: [&str; 32] = [
    "Arizona Cardinals",
    "Atlanta Falcons",
    "Baltimore Ravens",
    "Buffalo Bills",
    "Carolina Panthers",
    "Chicago Bears",
    "Cincinnati Bengals",
    "Cleveland Browns",
    "Dallas Cowboys",
    "Denver Broncos",
    "Detroit Lions",
    "Green Bay Packers",
    "Houston Texans",
    "Indianapolis Colts",
    "Jacksonville Jaguars",
    "Kansas City Chiefs",
    "Las Vegas Raiders",
    "Los Angeles Chargers",
    "Los Angeles Rams",
    "Miami Dolphins",
    "Minnesota Vikings",
    "New England Patriots",
    "New Orleans Saints",
    "New York Giants",
    "New York Jets",
    "Philadelphia Eagles",
    "Pittsburgh Steelers",
    "San Francisco 49ers",
    "Seattle Seahawks",
    "Tampa Bay Buccaneers",
    "Tennessee Titans",
    "Washington Commanders",
];

/// Depth chart slots per position on a 53-man roster: starters, the extra
/// players who rotate in on most snaps (third receiver, nickel corner, a
/// rotational pass rusher), and the total kept on the roster. One kicker and
/// one punter.
#[derive(Clone, Copy, Debug)]
pub struct DepthSlots {
    pub starters: usize,
    pub rotation: usize,
    pub roster: usize,
}

pub fn depth_slots(pos: Position) -> DepthSlots {
    let (starters, rotation, roster) = match pos {
        Position::QB => (1, 0, 3),
        Position::RB => (1, 1, 4),
        Position::WR => (3, 1, 6),
        Position::TE => (1, 1, 3),
        Position::OT => (2, 0, 4),
        Position::IOL => (3, 0, 5),
        Position::EDGE => (2, 1, 5),
        Position::DT => (2, 1, 5),
        Position::LB => (2, 1, 5),
        Position::CB => (2, 1, 6),
        Position::S => (2, 0, 4),
        Position::K | Position::P => (1, 0, 1),
    };
    DepthSlots { starters, rotation, roster }
}

/// Practice squad places per team.
pub const PRACTICE_SQUAD: usize = 16;

/// A player on a depth chart: a background veteran, or one of the class (with its record index).
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub rating: f64,
    pub age: u32,
    /// Set for a player from the simulated class: his record index.
    pub id: Option<u32>,
    /// Role and depth chart spot from the last camp.
    pub role: Option<PlayingTime>,
    pub depth: Option<String>,
}

impl Member {
    pub fn background(rating: f64, age: u32) -> Self {
        Member { rating, age, id: None, role: None, depth: None }
    }
    #[inline]
    fn on_roster(&self) -> bool {
        self.role != Some(PlayingTime::PracticeSquad)
    }
}

#[derive(Clone, Debug)]
pub struct Team {
    pub id: usize,
    pub name: &'static str,
    /// Front office judgment (the GM), 1-20: how clearly the team sees players.
    pub draft_skill: f64,
    /// Owner, 1-20: patience, spending and stability.
    pub owner: f64,
    /// Head coach and staff, 1-20: how well players develop there.
    pub coaching: f64,
    pub depth: [Vec<Member>; POSITION_COUNT],
    /// Years the current coaching staff has been in place.
    pub staff_years: u32,
    /// In the world, the named head coach and coordinators (ids into the
    /// world's coaches; dynasty item 11.12). The class sim leaves them empty
    /// and keeps its own rough staff changes.
    pub head_coach: Option<usize>,
    pub oc: Option<usize>,
    pub dc: Option<usize>,
    /// Win shares of the world's NFL seasons, most recent last (11.12).
    pub win_share: Vec<f64>,
}

impl Team {
    /// How well run a team is, 1-20: GM, owner and coaching staff together.
    pub fn organisation(&self) -> f64 {
        (self.draft_skill + self.owner + self.coaching) / 3.0
    }

    /// Ratings of the roster (not practice squad) at a position, best first.
    fn roster_ratings(&self, pos: Position) -> Vec<(f64, u32)> {
        let mut v: Vec<(f64, u32)> =
            self.depth[pos.index()].iter().filter(|m| m.on_roster()).map(|m| (m.rating, m.age)).collect();
        v.sort_by(|a, b| b.0.total_cmp(&a.0));
        v
    }

    /// Average rating of the team's starters: roughly how good the team is this year.
    pub fn strength(&self) -> f64 {
        let mut sum = 0.0;
        let mut n = 0;
        for pos in Position::ALL {
            for (rating, _) in self.roster_ratings(pos).into_iter().take(depth_slots(pos).starters) {
                sum += rating;
                n += 1;
            }
        }
        if n > 0 { sum / n as f64 } else { 0.0 }
    }

    /// Weakest starter at a position (the spot a team most wants to fill): his rating and age.
    pub fn weakest_starter(&self, pos: Position) -> Option<(f64, u32)> {
        let best = self.roster_ratings(pos);
        let n = best.len().min(depth_slots(pos).starters);
        if n == 0 { None } else { Some(best[n - 1]) }
    }
}

/// Age at which a background veteran at a position typically leaves the game.
fn background_exit_age(pos: Position) -> f64 {
    match pos {
        Position::QB => 36.0,
        Position::RB => 29.0,
        Position::WR | Position::LB | Position::CB | Position::S => 31.0,
        Position::TE | Position::EDGE | Position::DT => 32.0,
        Position::OT | Position::IOL => 33.0,
        Position::K | Position::P => 38.0,
    }
}

/// Yearly rating change of a background player with age.
fn background_ageing(pos: Position, age: u32) -> f64 {
    let shift = match pos {
        Position::RB => -2.0,
        Position::CB | Position::S => -1.0,
        Position::QB => 3.0,
        Position::K | Position::P => 5.0,
        _ => 0.0,
    };
    let a = age as f64 - shift;
    if a < 25.0 {
        2.0
    } else if a < 28.0 {
        0.6
    } else if a < 30.0 {
        -0.6
    } else {
        -(1.4 + 0.7 * (a - 30.0))
    }
}

fn new_background(rng: &mut Rng, pos: Position, slot: usize, talent: f64, young: bool) -> Member {
    let DepthSlots { starters, rotation, .. } = depth_slots(pos);
    let mean = if slot < starters {
        78.0 + 3.0 * talent
    } else if slot < starters + rotation {
        73.0 + 2.0 * talent
    } else {
        69.0 + 1.5 * talent
    };
    let sd = if slot < starters { 4.5 } else { 3.5 };
    let age = if young {
        rng.normal(23.5, 1.5)
    } else if pos.is_specialist() {
        rng.normal(30.0, 4.0)
    } else {
        rng.normal(27.0, 3.0)
    };
    let rating = clamp(rng.normal(mean - if young { 2.0 } else { 0.0 }, sd), 55.0, 97.0);
    Member::background(rating, round_half_up(clamp(age, 21.0, 38.0)) as u32)
}

fn staff_rating(rng: &mut Rng, mean: f64, sd: f64) -> f64 {
    clamp(round_half_up(rng.normal(mean, sd)), 1.0, 20.0)
}

pub fn make_league(rng: &mut Rng) -> Vec<Team> {
    NFL_TEAM_NAMES
        .iter()
        .enumerate()
        .map(|(id, &name)| {
            let draft_skill = staff_rating(rng, 10.5, 3.5);
            let owner = staff_rating(rng, 10.5, 3.5);
            let coaching = staff_rating(rng, 13.0, 3.0);
            let staff_years = rng.int(0, 5) as u32;
            let mut team = Team {
                id,
                name,
                draft_skill,
                owner,
                coaching,
                depth: Default::default(),
                staff_years,
                head_coach: None,
                oc: None,
                dc: None,
                win_share: Vec::new(),
            };
            let talent = talent_of(&team, rng);
            for pos in Position::ALL {
                team.depth[pos.index()] =
                    (0..depth_slots(pos).roster).map(|slot| new_background(rng, pos, slot, talent, false)).collect();
            }
            team
        })
        .collect()
}

/// Roster talent a team tends to acquire: well-run teams find better players, with plenty of luck.
fn talent_of(team: &Team, rng: &mut Rng) -> f64 {
    0.5 * rng.normal(0.0, 1.0) + 0.5 * ((team.organisation() - 11.0) / 3.0)
}

/// One offseason for the background players: they age, develop or decline,
/// and leave (retirement, released, signed elsewhere); their places are filled
/// by other classes' rookies and free agents. Staffs get fired, more often
/// after bad years.
pub fn advance_league(teams: &mut [Team], rng: &mut Rng) {
    let mut strengths: Vec<f64> = teams.iter().map(Team::strength).collect();
    strengths.sort_by(f64::total_cmp);
    let bottom = strengths[7];
    for team in teams.iter_mut() {
        let weak = team.strength() <= bottom;
        team.staff_years += 1;
        // A world team with named coaches changes staff in the coaching carousel.
        let new_staff = rng.chance(if weak { 0.3 } else { 0.1 });
        if new_staff && team.head_coach.is_none() {
            team.coaching = staff_rating(rng, 13.0, 3.0);
            team.staff_years = 0;
        }
        if rng.chance(if weak { 0.15 } else { 0.05 }) {
            team.draft_skill = staff_rating(rng, 10.5, 3.5);
        }
        let talent = talent_of(team, rng);
        for pos in Position::ALL {
            let mut ranked = std::mem::take(&mut team.depth[pos.index()]);
            ranked.sort_by(|x, y| y.rating.total_cmp(&x.rating));
            let mut kept: Vec<Member> = Vec::with_capacity(ranked.len() + 2);
            let mut open: Vec<usize> = Vec::new();
            for (slot, mut m) in ranked.into_iter().enumerate() {
                if m.id.is_some() {
                    kept.push(m);
                    continue;
                }
                m.age += 1;
                m.rating = clamp(m.rating + background_ageing(pos, m.age) + rng.normal(0.0, 2.0), 40.0, 97.0);
                let leaves = m.age as f64 > background_exit_age(pos) + rng.normal(0.0, 2.0)
                    || m.rating < 60.0
                    || rng.chance(0.12);
                if leaves {
                    open.push(slot);
                } else {
                    kept.push(m);
                }
            }
            // Fill back to a full depth chart. A team that loses a starter often
            // replaces him with someone a level down.
            let roster = depth_slots(pos).roster;
            // Practice squad players don't hold a place on the 53.
            let mut on_roster = kept.iter().filter(|m| m.on_roster()).count();
            let mut i = 0;
            while on_roster < roster {
                let slot = open.get(i).copied().unwrap_or(roster - 1);
                let s = if rng.chance(0.65) { slot } else { (slot + 1).min(roster - 1) };
                let young = rng.chance(0.6);
                kept.push(new_background(rng, pos, s, talent, young));
                on_roster += 1;
                i += 1;
            }
            team.depth[pos.index()] = kept;
        }
    }
}
