//! The fact sheet (talk facts design, sections 3, 4 and 14;
//! `/afm/talk-facts/proposal.md` in the project folder): one reader of the
//! world for every talk. Before a talk opens, the tree asks for a sheet of
//! talking points for the person across from you and for you, and builds
//! its options, its lines' facts and its sizes from it.
//!
//! - `kinds`: the catalogue, one row a kind of point (key, whose, when it
//!   can be checked, window, size).
//! - `sheet`: `Point`, `Sheet`, your proof's `Tier`, and the sizes every
//!   talk shares (`Basis`).
//! - `player`, `season`: a player on a team, in season and out.
//! - `program`, `recruit`: your points to a recruit or a transfer with
//!   your proof graded against his other schools, and what changed for him
//!   and what is coming.
//! - `team`: your team's week (the team talk, the press, your boss).
//! - `alumni`: the one new index, NFL alumni by program and position.
//! - `memory`: what you have talked through with each person (`Talked`,
//!   kept in `Folk.talked`).
//! - `words`: words for a day, a count, a box score.
//!
//! The live talks' claim checker lives in this module too (section 14):
//! a typed fact is checked against the same sheet.
//!
//! # The hidden numbers
//!
//! A sheet is read by lines and screens, so nothing hidden may sit in a
//! point's values or checks: not his true potential, his grades, the size
//! of a growth step, his portal lean, the services' read, his weekly growth
//! state. The readers here keep to three rules, and `tests/claims.rs`
//! checks them:
//!
//! 1. A value is a name, a date in words, a whole count, a box score or a
//!    measured number (`Point::text`, `count`, `measured`).
//! 2. A hidden number is read only as a gate (which side of a line he is
//!    on) and the line with the read is marked `// hidden:`.
//! 3. What only he knows goes in `Point::hide`, which no line can reach,
//!    and a point you don't know is marked so (`Point::known`).
//!
//! `Point.mind` and `Point.proof` are engine numbers: a tree picks and
//! sizes with them and shows neither.
//!
//! Every size in this module is Claude's (proposed, not confirmed).

mod alumni;
pub mod kinds;
mod memory;
mod player;
mod program;
mod recruit;
pub mod rooms;
mod season;
pub mod sheet;
mod team;
pub mod words;

pub use alumni::{Alum, Alumni};
pub use kinds::{Family, Group, Kind, When, Whose};
pub use memory::{KEEP_DAYS, Row, Talked};
pub use program::Ear;
pub use sheet::{Basis, Conduct, Point, Proof, Ref, Sheet, Tier};

use crate::calendar::Date;
use crate::player::Trait;
use crate::positions::Position;
use crate::world::folk::{Team, facts};
use crate::world::seat::SeatGame;
use crate::world::{Stage, World};
use std::cell::OnceCell;
use std::collections::HashMap;

/// How far a trait moves how much he cares about a point: a man at 20
/// cares this much more than an average one, a man at 0 this much less.
/// Proposed, not confirmed.
pub const CARE_SWING: f32 = 0.3;

/// A team's games: (day, opponent, at home).
type Games = Vec<(Date, Team, bool)>;
/// A depth chart: each spot's players, the starter first.
type Depth = Vec<(Position, Vec<u32>)>;
/// Who joined a program: (his place in the people, the day, signed).
type Joined = Vec<(usize, Date, bool)>;

/// One pass over the world: the day, and what several readers would
/// otherwise work out twice. Build one for a sheet, or one for a whole
/// roster's sheets.
pub(crate) struct Reader<'a> {
    pub w: &'a World,
    pub today: Date,
    games: OnceCell<(Team, Games)>,
    depth: OnceCell<Option<Depth>>,
    rates: OnceCell<HashMap<u32, f64>>,
    joined: OnceCell<(usize, Joined)>,
}

impl<'a> Reader<'a> {
    pub fn new(w: &'a World) -> Reader<'a> {
        Reader {
            w,
            today: w.today,
            games: OnceCell::new(),
            depth: OnceCell::new(),
            rates: OnceCell::new(),
            joined: OnceCell::new(),
        }
    }

    /// A person's name by his id.
    pub fn name(&self, id: u32) -> Option<String> {
        crate::world::folk::person(self.w, id).map(|i| self.w.people[i].name.clone())
    }

    /// A team's games this season: (day, opponent, at home), oldest first.
    pub fn games(&self, team: Team) -> Vec<(Date, Team, bool)> {
        let (t, v) = self.games.get_or_init(|| (team, facts::team_games(self.w, team)));
        if *t == team { v.clone() } else { facts::team_games(self.w, team) }
    }

    /// Whether a team is in its season today: from the week of its first
    /// game to a week after its last.
    pub fn in_season(&self, team: Team) -> bool {
        let g = self.games(team);
        match (g.first(), g.last()) {
            (Some(a), Some(b)) => self.today >= a.0.add(-kinds::GAME_DAYS) && self.today <= b.0.add(kinds::GAME_DAYS),
            _ => false,
        }
    }

    /// The depth chart your team would take into a game today.
    pub fn depth(&self) -> Option<&Vec<(Position, Vec<u32>)>> {
        self.depth.get_or_init(|| self.w.seat_game_depth()).as_ref()
    }

    /// The going rate for one of your college players: the usual share of
    /// the school's money and of the collective's for his spot and role.
    pub fn going_rate(&self, id: u32) -> Option<f64> {
        let Some(Team::College(j)) = Team::seat(self.w) else { return None };
        self.rates
            .get_or_init(|| {
                use crate::world::front_office::college as desk;
                let mut m: HashMap<u32, f64> = desk::usual_deals(self.w, j).into_iter().collect();
                for (id, deal, _) in desk::collective_deals(self.w, j) {
                    *m.entry(id).or_default() += deal;
                }
                m
            })
            .get(&id)
            .copied()
    }

    /// Who committed to a program or signed with it in the last month:
    /// (his place in the people, the day, whether he signed). One pass
    /// over the world, kept for the program asked first.
    pub fn joined(&self, prog: usize) -> Vec<(usize, Date, bool)> {
        let scan = |prog: usize| -> Vec<(usize, Date, bool)> {
            let fresh = |d: Date| (0..=kinds::NEWS_DAYS).contains(&d.days_to(self.today));
            self.w
                .people
                .iter()
                .enumerate()
                .filter_map(|(i, p)| {
                    if let Some(s) = p.cal.signed.filter(|s| s.program == prog && !s.walk_on) {
                        return fresh(s.date).then_some((i, s.date, true));
                    }
                    let (j, day, walk_on) = p.cal.commit?;
                    (j == prog && !walk_on && fresh(day)).then_some((i, day, false))
                })
                .collect()
        };
        let (p, v) = self.joined.get_or_init(|| (prog, scan(prog)));
        if *p == prog { v.clone() } else { scan(prog) }
    }

    /// What one of your college players is paid: your deals with him.
    pub fn deal(&self, id: u32) -> f64 {
        crate::world::front_office::college::deal_of(self.w, id).map_or(0.0, |d| d.0 + d.1)
    }
}

/// The person a sheet is about.
pub(crate) struct Man {
    /// His place in `World.people` today.
    pub i: usize,
    pub id: u32,
    pub team: Option<Team>,
    /// On your team: the stores about your own team speak for him.
    pub mine: bool,
}

impl Man {
    pub fn of(w: &World, i: usize) -> Man {
        let team = team_of(w, i);
        Man { i, id: w.people[i].player.id, team, mine: team.is_some() && team == Team::seat(w) }
    }
}

/// The team a player is on today.
pub fn team_of(w: &World, i: usize) -> Option<Team> {
    let p = &w.people[i];
    match p.stage {
        Stage::College => p.college.map(Team::College),
        Stage::Pro => {
            p.pro.as_ref().and_then(|c| c.team).map(Team::Nfl).or_else(|| w.others.team_of(i).map(Team::Other))
        }
        Stage::HighSchool => Some(Team::High(p.hs)),
        _ => None,
    }
}

/// Which side of a kept game a team was (0 home, 1 away).
pub(crate) fn side_of(g: &SeatGame, team: Team) -> Option<usize> {
    let (_, i) = team.parts();
    if g.level != team.level() || matches!(team, Team::Other(_)) {
        return None;
    }
    g.teams.iter().position(|&t| t == i)
}

/// The same kind of team as `like`, by its index (an opponent in a kept
/// game).
pub(crate) fn same_kind(like: Team, i: usize) -> Team {
    match like {
        Team::College(_) => Team::College(i),
        Team::Nfl(_) => Team::Nfl(i),
        Team::High(_) => Team::High(i),
        Team::Other(_) => Team::Other(i),
    }
}

/// The trait that moves how much a man cares about a kind of point, and
/// which way: an ambitious man cares more about his job, a loyal one about
/// a friend who left, one who adapts badly about a new system.
fn cares(kind: Kind) -> Option<(Trait, f32)> {
    use Kind as K;
    Some(match kind {
        K::LostJob | K::Buried | K::WonJob | K::Slump | K::ChartMove | K::RoomChanged | K::PracticeWeek => {
            (Trait::Ambition, 1.0)
        }
        K::Draft | K::WantsOut | K::Portal | K::Colleges | K::ProSpring => (Trait::Ambition, 1.0),
        K::Pay => (Trait::Greed, 1.0),
        K::Award | K::BigGame | K::BadGame => (Trait::Ego, 1.0),
        K::PromiseBroken | K::PromiseOpen => (Trait::Temper, 1.0),
        K::FriendLeft | K::TeammateDrafted => (Trait::Loyalty, 1.0),
        K::TestingDay | K::ReportDay | K::Effort | K::Grades | K::FailedTerm => (Trait::Professionalism, 1.0),
        K::Homesick | K::NewSystem | K::NewPosition | K::NewCoach => (Trait::Adaptability, -1.0),
        _ => return None,
    })
}

/// Finishes a sheet: drops what has left its window (rule 9), then sets
/// how much each point is on his mind today: how big it is (the kind's
/// size and the reader's own weight), how fresh, how much he cares, and
/// whether you have already talked it through.
fn seal(r: &Reader, about: Option<usize>, team: Option<Team>, points: Vec<Point>) -> Sheet {
    let w = r.w;
    let id = about.map(|i| w.people[i].player.id);
    // What you have talked through is your memory: it fades a point only
    // on a sheet built for your own side.
    let yours = team.is_some() && team == Team::seat(w);
    let mut sheet = Sheet::new(r.today, id, team);
    for mut p in points {
        let Some(fresh) = sheet::fresh(p.kind, p.day, r.today) else { continue };
        let mut by = fresh;
        if let Some(id) = id.filter(|_| yours) {
            by *= sheet::TALKED_FADE.powi(w.folk.talked.times(r.today, id, &p) as i32);
        }
        if let (Some(i), Some((t, sign))) = (about, cares(p.kind)) {
            p = p.care(1.0 + sign * CARE_SWING * w.people[i].player.personality.traits.z(t) as f32);
        }
        sheet.push(Some(p.weigh(by)));
    }
    sheet.settle()
}

/// The sheet for a talk with a player, yours or another team's: what is
/// true for him today, the point most on his mind first. An AI coach's
/// talk with his own player reads the same sheet (rule 8); the stores the
/// world keeps only for your team (your rulings, your promises, the games
/// kept whole, practice) speak only for your players.
pub fn player(w: &World, i: usize) -> Sheet {
    let r = Reader::new(w);
    player_with(&r, i)
}

/// A sheet for everyone on a team, one pass over the world.
pub fn roster(w: &World, team: Team) -> Vec<Sheet> {
    let r = Reader::new(w);
    team.roster(w).into_iter().map(|i| player_with(&r, i)).collect()
}

fn player_with(r: &Reader, i: usize) -> Sheet {
    let m = Man::of(r.w, i);
    let mut points = Vec::new();
    player::points(r, &m, &mut points);
    season::points(r, &m, &mut points);
    seal(r, Some(i), m.team, points)
}

/// The sheet for a call, a message or a visit with a recruit or a
/// transfer, for the program `prog`: your points to him with your proof
/// graded against the same fact at his other top schools, what changed
/// for him since you last spoke, and what is coming. `ear` is who listens:
/// himself on a call, whoever decides at his home.
pub fn recruit(w: &World, i: usize, prog: usize, ear: Ear) -> Sheet {
    let r = Reader::new(w);
    let mut points = Vec::new();
    program::points(&r, i, prog, ear, &mut points);
    recruit::points(&r, i, prog, &mut points);
    seal(&r, Some(i), Some(Team::College(prog)), points)
}

/// Your team's own sheet: its last game while it is fresh, a streak, the
/// next game, the starters out, the season that ended (the team talk, the
/// press, your boss).
pub fn team(w: &World, team: Team) -> Sheet {
    let r = Reader::new(w);
    let mut points = Vec::new();
    team::points(&r, team, &mut points);
    seal(&r, None, Some(team), points)
}
