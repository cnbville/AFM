//! New teams after 2026 (SK68): NFL expansion clubs (`nfl`), colleges
//! that start or restart football (`college`) and new high schools in
//! growing places (`school`), each at its real rate, and the build year of
//! a team that doesn't play yet. A ground zero start or a later job puts
//! you at one of them (`ground_zero`).

pub mod college;
pub mod ground_zero;
pub mod nfl;
pub mod school;

use crate::world::World;

/// The colleges and high schools that joined after 2026.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NewTeams {
    pub programs: Vec<NewProgram>,
    pub schools: Vec<NewSchool>,
}

/// A college that starts or restarts football: the season it said so, its
/// build year (a team that practices and plays no games) and its first
/// season of games.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NewProgram {
    pub program: usize,
    pub announced: i32,
    pub build: i32,
    pub first: i32,
    /// Coming back after a shutdown (UAB in 2017), not a first team.
    pub restart: bool,
}

/// A new high school: the season it opens and the schools whose zones it
/// takes a part of.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct NewSchool {
    pub school: usize,
    pub announced: i32,
    pub opens: i32,
    pub parents: Vec<usize>,
    /// Its zone's players have moved in (the first school day).
    pub zoned: bool,
    /// Players who moved in from each parent school, and those who chose to
    /// stay.
    #[serde(default)]
    pub moved: u32,
    #[serde(default)]
    pub stayed: u32,
    /// Zoned juniors the new school's coach has visited (your pitch).
    #[serde(default)]
    pub pitched: Vec<u32>,
}

impl NewTeams {
    pub fn program(&self, p: usize) -> Option<&NewProgram> {
        self.programs.iter().find(|x| x.program == p)
    }

    /// A program in its build year: it practices and plays no games.
    pub fn build_year(&self, p: usize, season: i32) -> bool {
        self.programs.iter().any(|x| x.program == p && season < x.first && season >= x.build)
    }

    pub fn school(&self, h: usize) -> Option<&NewSchool> {
        self.schools.iter().find(|x| x.school == h)
    }
}

impl World {
    /// Whether a program plays games this season: it fields a team and
    /// isn't in its build year (SK68).
    pub fn fields_games(&self, p: usize, season: i32) -> bool {
        self.programs.get(p).is_some_and(|x| x.fields_games(season))
    }
}

/// The football year's start after 2026: colleges announce football at
/// their real rate, districts open new high schools where the young
/// population grows. Called before the year's season states are built.
pub(crate) fn year_start(w: &mut World) {
    if w.year <= crate::world::franchise::PRESENT {
        return;
    }
    college::year(w);
    school::year(w);
}
