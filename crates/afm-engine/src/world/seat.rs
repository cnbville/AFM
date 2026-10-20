//! Your seat in the world (section 8): what the game keeps for your own
//! team beyond what every AI team has. Your games' full box scores and play
//! logs, your depth chart, and plain reads of the season state the screens
//! need. Nothing here changes an AI team; a world without a seat plays
//! exactly as before.

use crate::calendar::Date;
use crate::game::GameStats;
use crate::positions::Position;

use super::World;

/// One of your team's games, kept whole for its game page: the box score,
/// every play with the caller's reasons, and each side's name that day.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SeatGame {
    pub date: Date,
    pub level: crate::calendar::Level,
    /// High school, college program or NFL club index of each side (home, away).
    pub teams: [usize; 2],
    pub names: [String; 2],
    pub stats: GameStats,
}

/// How many of your games are kept whole (about two seasons and a
/// postseason); older ones keep their score in the season records.
const KEEP_GAMES: usize = 40;

impl World {
    /// Keeps a game whole when it is your team's.
    pub(crate) fn keep_seat_game(&mut self, level: crate::calendar::Level, teams: [usize; 2], stats: &GameStats) {
        let Some(seat) = self.seat else { return };
        let mine = match level {
            crate::calendar::Level::Nfl => seat.nfl_team.is_some_and(|t| teams.contains(&t)),
            _ => seat.program.is_some_and(|p| teams.contains(&p)),
        };
        if !mine || seat.level != level {
            return;
        }
        let names = match level {
            crate::calendar::Level::Nfl => teams.map(|t| self.nfl_team_name(t)),
            crate::calendar::Level::HighSchool => teams.map(|h| self.school_name(h).to_string()),
            _ => teams.map(|p| self.program_name(p).to_string()),
        };
        self.seat_games.push(SeatGame { date: self.today, level, teams, names, stats: stats.clone() });
        if self.seat_games.len() > KEEP_GAMES {
            let cut = self.seat_games.len() - KEEP_GAMES;
            self.seat_games.drain(..cut);
        }
    }

    /// An NFL club's name in this season (its franchise by era).
    pub fn nfl_team_name(&self, t: usize) -> String {
        self.nfl.get(t).map_or_else(|| format!("Team {t}"), |team| team.name.to_string())
    }

    /// Your depth chart order at a position (player ids, best first).
    pub fn set_seat_depth(&mut self, pos: Position, ids: Vec<u32>) {
        self.seat_depth.retain(|(p, _)| *p != pos);
        if !ids.is_empty() {
            self.seat_depth.push((pos, ids));
        }
    }

    /// Sets (or with None clears) a player's plan focus.
    pub fn set_focus(&mut self, player: u32, focus: Option<crate::attributes::Attr>) {
        self.seat_focus.retain(|(id, _)| *id != player);
        if let Some(a) = focus {
            self.seat_focus.push((player, a.index() as u8));
        }
    }

    pub fn focus_of(&self, player: u32) -> Option<crate::attributes::Attr> {
        self.seat_focus.iter().find(|(id, _)| *id == player).map(|&(_, a)| crate::attributes::Attr::ALL[a as usize])
    }

    /// The plan focus of each player on your roster now, by person index
    /// (a plan stays with your team: a player who leaves drops it).
    pub(crate) fn focus_by_person(&self) -> std::collections::HashMap<usize, crate::attributes::Attr> {
        let Some(seat) = self.seat else { return Default::default() };
        if self.seat_focus.is_empty() {
            return Default::default();
        }
        let roster: &[usize] = match seat.level {
            crate::calendar::Level::Nfl => seat.nfl_team.and_then(|t| self.clubs.get(t)).map_or(&[], |c| &c.roster),
            crate::calendar::Level::HighSchool => seat.program.map_or(&[], |h| &self.high_schools[h].roster),
            _ => seat.program.map_or(&[], |p| &self.programs[p].roster),
        };
        roster.iter().filter_map(|&i| self.focus_of(self.people[i].player.id).map(|a| (i, a))).collect()
    }

    /// Whether a college program is yours (its game team takes your order).
    pub(crate) fn seat_program(&self, prog: usize) -> bool {
        self.seat.is_some_and(|s| s.program == Some(prog) && s.level == crate::calendar::Level::College)
    }

    /// Your high school, when you coach one.
    pub(crate) fn seat_school(&self) -> Option<usize> {
        self.seat.filter(|s| s.level == crate::calendar::Level::HighSchool).and_then(|s| s.program)
    }

    pub(crate) fn seat_club(&self, t: usize) -> bool {
        self.seat.is_some_and(|s| s.nfl_team == Some(t) && s.level == crate::calendar::Level::Nfl)
    }
}

/// A college team's season so far, for the screens.
#[derive(Clone, Copy, Debug, Default)]
pub struct Record {
    pub wins: u32,
    pub losses: u32,
    pub conf_wins: u32,
    pub conf_losses: u32,
}

/// A college game on this season's schedule, for the screens.
#[derive(Clone, Debug)]
pub struct Fixture {
    pub date: Date,
    pub kickoff: (u16, u16),
    pub home: usize,
    pub away: usize,
    pub name: Option<&'static str>,
    pub conference: bool,
    pub neutral: bool,
    pub why: Vec<&'static str>,
    /// Home and away points, once played.
    pub score: Option<[u32; 2]>,
}

/// An NFL game on this season's schedule.
#[derive(Clone, Debug)]
pub struct NflFixture {
    pub week: usize,
    pub home: usize,
    pub away: usize,
    pub when: Option<Date>,
    pub played: bool,
}

impl World {
    /// This college season's record for a program.
    pub fn college_record(&self, prog: usize) -> Record {
        self.college.as_ref().and_then(|cs| cs.results.get(prog)).map_or(Record::default(), |r| Record {
            wins: r.wins,
            losses: r.losses,
            conf_wins: r.conf_wins,
            conf_losses: r.conf_losses,
        })
    }

    /// This college season's games for a program (all programs if None).
    pub fn college_fixtures(&self, prog: Option<usize>) -> Vec<Fixture> {
        let Some(cs) = self.college.as_ref() else { return Vec::new() };
        cs.games
            .iter()
            .filter(|g| prog.is_none_or(|p| g.home == p || g.away == p))
            .map(|g| Fixture {
                date: g.date,
                kickoff: g.kickoff,
                home: g.home,
                away: g.away,
                name: g.name,
                conference: g.conference,
                neutral: g.site.is_some(),
                why: g.why.clone(),
                score: g.played.filter(|p| !p.cancelled).map(|p| p.score),
            })
            .collect()
    }

    /// The college season's year, if one is under way or just finished.
    pub fn college_year(&self) -> Option<i32> {
        self.college.as_ref().map(|cs| cs.year)
    }

    /// This NFL season's record (wins, losses, ties) for a club.
    pub fn nfl_record(&self, t: usize) -> (u32, u32, u32) {
        self.nfl_season.as_ref().map_or((0, 0, 0), |s| {
            (
                s.wins.get(t).copied().unwrap_or(0),
                s.losses.get(t).copied().unwrap_or(0),
                s.ties.get(t).copied().unwrap_or(0),
            )
        })
    }

    /// Conference (0 AFC, 1 NFC) and division (0-7) of each club this season.
    pub fn nfl_alignment(&self) -> Option<(Vec<usize>, Vec<usize>)> {
        self.nfl_season.as_ref().map(|s| (s.conf.clone(), s.div.clone()))
    }

    /// This NFL season's games for a club (all if None).
    pub fn nfl_fixtures(&self, t: Option<usize>) -> Vec<NflFixture> {
        let Some(s) = self.nfl_season.as_ref() else { return Vec::new() };
        let mut out = Vec::new();
        for (k, week) in s.weeks.iter().enumerate() {
            for (g, &(h, a)) in week.iter().enumerate() {
                if t.is_some_and(|t| h != t && a != t) {
                    continue;
                }
                let slot = s.slots.get(k).and_then(|w| w.get(g));
                out.push(NflFixture {
                    week: k + 1,
                    home: h,
                    away: a,
                    when: slot.map(|x| x.at.date),
                    played: slot.is_some_and(|x| x.played),
                });
            }
        }
        out
    }

    /// Your team's games kept whole, oldest first.
    pub fn seat_games(&self) -> &[SeatGame] {
        &self.seat_games
    }
}

/// You, as the setup builds you (section 8, topic 2; people topic 15).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct You {
    pub name: String,
    pub age: u32,
    pub ethnicity: super::ethnicity::Ethnicity,
    pub home_state: usize,
    pub ex_pro: bool,
    pub reputation: f64,
    /// Your ratings, 1-20, on the same scale as every coach.
    pub teaching: [f64; super::coach::GROUP_COUNT],
    pub scheme: f64,
    pub game_management: f64,
    pub motivation: f64,
    pub discipline: f64,
    pub recruiting: f64,
    pub network: f64,
    pub relationships: f64,
    pub style: crate::tendencies::Style,
}

/// Where you start (section 8, topic 2).
#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Job {
    HighSchool(usize),
    College(usize),
    Nfl(usize),
}

impl World {
    /// Starts your career on this world (section 8, topics 1 and 2): you
    /// take the job, the clock stops for your team's moments, and the world
    /// gets fresh randomness from today, so two careers from one library
    /// world share its history and then play out differently. The career's
    /// seed makes it replay exactly. Returns your coach id.
    pub fn start_career(&mut self, you: &You, job: Job, career_seed: u32) -> usize {
        // Fresh randomness: the world's own stream, the seed every keyed
        // draw reads, and every person's stream, each mixed with the career.
        self.options.seed ^= career_seed.rotate_left(13) ^ 0x5EA7_0001;
        self.rng = crate::rng::Rng::new(self.options.seed ^ 0xC0FF_EE11);
        let seed = self.options.seed;
        self.people.iter_mut().for_each(|p| p.rng = crate::rng::Rng::new(seed ^ p.player.id.wrapping_mul(0x9E37_79B9)));
        let employer = match job {
            Job::HighSchool(s) => super::coach::Employer::HighSchool(s),
            Job::College(p) => super::coach::Employer::College(p),
            Job::Nfl(t) => super::coach::Employer::Nfl(t),
        };
        let id = super::staff::take_seat(self, you, employer);
        self.seat = Some(match job {
            Job::HighSchool(s) => {
                super::Seat { level: crate::calendar::Level::HighSchool, nfl_team: None, program: Some(s) }
            }
            Job::College(p) => super::Seat { level: crate::calendar::Level::College, nfl_team: None, program: Some(p) },
            Job::Nfl(t) => super::Seat { level: crate::calendar::Level::Nfl, nfl_team: Some(t), program: None },
        });
        self.career = Some(Career {
            you: you.clone(),
            coach: id,
            job,
            started: self.today,
            seed: career_seed,
            notes: Vec::new(),
            hs_staff: Vec::new(),
        });
        if let Job::HighSchool(h) = job {
            let staff = self.hire_hs_staff(h);
            if let Some(c) = self.career.as_mut() {
                c.hs_staff = staff;
            }
        }
        self.seat_depth.clear();
        self.seat_games.clear();
        self.seat_focus.clear();
        id
    }
}

/// Your career (section 8): who you are, your coach in the world, where you
/// started and your own notes on people (screens topic 5: private, they
/// change nothing in the world and follow the person through his career).
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Career {
    pub you: You,
    pub coach: usize,
    pub job: Job,
    pub started: Date,
    pub seed: u32,
    /// Notes by player id, oldest first.
    pub notes: Vec<(u32, Vec<Note>)>,
    /// Your high school staff, when you coach a high school.
    #[serde(default)]
    pub hs_staff: Vec<HsAssistant>,
}

/// A high school assistant (people topic 10): most teach at the school and
/// coach for a stipend. Small schools have two to four, big Texas programs
/// fifteen or more across varsity, JV and freshmen.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HsAssistant {
    pub name: String,
    pub age: u32,
    /// Offensive or defensive coordinator, or a position group's coach.
    pub coordinator: Option<bool>,
    /// The group he coaches (`coach::Group` as an index).
    pub group: u8,
    /// What he teaches in class, or None for a walk-on coach.
    pub subject: Option<String>,
    /// Coaching stipend a season, in dollars.
    pub stipend: u32,
    /// Hidden teaching ability, 1-20 on every coach's scale.
    pub teaching: f64,
    pub since: i32,
}

/// Subjects high school coaches teach (proposed, not confirmed: weighted
/// toward physical education and history, as real staffs are).
const SUBJECTS: [&str; 10] = [
    "physical education",
    "physical education",
    "history",
    "history",
    "health",
    "math",
    "science",
    "English",
    "special education",
    "driver's education",
];

impl World {
    /// Your high school staff from the school's size and money (proposed,
    /// not confirmed: two plus one per 400 students, 3 to 15; ability
    /// around what the school's resources buy, the stand-in AI schools use).
    pub(crate) fn hire_hs_staff(&mut self, h: usize) -> Vec<HsAssistant> {
        let school = &self.high_schools[h];
        let n = (2 + school.enrollment / 400).clamp(3, 15) as usize;
        let mean = 4.0 + 12.0 * school.resources;
        let rich = school.resources;
        let year = self.year;
        let mut rng = crate::rng::Rng::new(self.options.seed ^ 0x45AF_F000 ^ h as u32);
        // Coordinators first, then the groups a staff fills in order.
        let order: [(Option<bool>, u8); 15] = [
            (Some(true), 0),
            (Some(false), 6),
            (None, 3),
            (None, 4),
            (None, 6),
            (None, 5),
            (None, 2),
            (None, 1),
            (None, 0),
            (None, 7),
            (None, 3),
            (None, 4),
            (None, 6),
            (None, 2),
            (None, 5),
        ];
        order[..n]
            .iter()
            .map(|&(coordinator, group)| {
                let eth = super::ethnicity::coach_ethnicity(super::ethnicity::CoachPool::HighSchool, year, &mut rng);
                let age = 24 + (rng.next() * 34.0) as u32;
                let name = super::names::person_name(eth, year - age as i32, &mut rng);
                // Some states allow walk-on coaches who don't teach.
                let subject = (rng.next() > 0.15).then(|| SUBJECTS[(rng.next() * 10.0) as usize % 10].to_string());
                let lead = if coordinator.is_some() { 1.0 } else { 0.0 };
                HsAssistant {
                    name,
                    age,
                    coordinator,
                    group,
                    subject,
                    stipend: (2500.0 + 5500.0 * rich + 1500.0 * lead + rng.normal(0.0, 800.0)).max(1000.0) as u32,
                    teaching: crate::rng::clamp(mean + lead + rng.normal(0.0, 2.5), 1.0, 20.0),
                    since: year - (rng.next() * 8.0) as i32,
                }
            })
            .collect()
    }

    /// What your high school staff teaches a position group: its best man
    /// for the group, coordinators covering their side (people topic 10;
    /// proposed, not confirmed).
    pub(crate) fn hs_staff_teaching(&self, h: usize, group: super::coach::Group) -> Option<f64> {
        if self.seat_school() != Some(h) {
            return None;
        }
        let staff = &self.career.as_ref()?.hs_staff;
        let g = group as u8;
        let offense = matches!(g, 0..=3);
        staff
            .iter()
            .filter(|a| a.group == g || a.coordinator == Some(offense) && g != 7)
            .map(|a| if a.group == g { a.teaching } else { a.teaching - 1.5 })
            .max_by(f64::total_cmp)
    }
}

/// One of your notes: the game date it was written and its text.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Note {
    pub date: Date,
    pub text: String,
}

impl World {
    pub fn add_note(&mut self, player: u32, text: String) {
        let date = self.today;
        let Some(c) = self.career.as_mut() else { return };
        match c.notes.iter_mut().find(|(id, _)| *id == player) {
            Some((_, list)) => list.push(Note { date, text }),
            None => c.notes.push((player, vec![Note { date, text }])),
        }
    }

    pub fn delete_note(&mut self, player: u32, k: usize) {
        let Some(c) = self.career.as_mut() else { return };
        if let Some((_, list)) = c.notes.iter_mut().find(|(id, _)| *id == player)
            && k < list.len()
        {
            list.remove(k);
        }
    }

    pub fn notes_on(&self, player: u32) -> &[Note] {
        self.career
            .as_ref()
            .and_then(|c| c.notes.iter().find(|(id, _)| *id == player))
            .map_or(&[], |(_, l)| l.as_slice())
    }
}

impl World {
    /// Your own ratings: you see them (people topic 15); nobody else's.
    pub fn your_ability(&self) -> Option<&super::coach::Ability> {
        let c = self.career.as_ref()?;
        self.coaches.get(c.coach).map(|co| co.ability())
    }
}

impl World {
    /// Fills in, after a file loads, what the file's game didn't keep
    /// (section 8, the world importer): a world built before section 8 has
    /// no seat, no career and no kept games, which load as empty, and its
    /// people list gets the same room rule as a running world.
    pub fn after_load(&mut self) {
        let room = self.people.len() + self.people.len() / 3;
        self.people.shrink_to(room);
        self.seat_games.retain(|g| g.teams.iter().all(|&t| t < self.programs.len().max(self.nfl.len())));
    }
}
